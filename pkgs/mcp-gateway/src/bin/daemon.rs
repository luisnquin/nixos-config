use std::collections::HashMap;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use clap::Parser;
use nyx_mcp_gateway::bridge::Bridge;
use nyx_mcp_gateway::config::{Config, Scope, Server};
use nyx_mcp_gateway::protocol::Subscription;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, mpsc};
use tokio::time::sleep;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    socket: Option<PathBuf>,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct InstanceKey {
    server: String,
    workspace: Option<PathBuf>,
}

struct Gateway {
    servers: HashMap<String, Server>,
    instances: Mutex<HashMap<InstanceKey, Arc<Bridge>>>,
    next_client_id: AtomicU64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let args = Args::parse();
    let config = Config::load(&args.config)?;
    let socket = args.socket.unwrap_or_else(default_socket);
    prepare_socket(&socket).await?;
    let listener = UnixListener::bind(&socket)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    let gateway = Arc::new(Gateway {
        servers: config.servers,
        instances: Mutex::new(HashMap::new()),
        next_client_id: AtomicU64::new(1),
    });

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let gateway = Arc::clone(&gateway);
                tokio::spawn(async move {
                    if let Err(error) = handle_client(stream, gateway).await {
                        tracing::warn!(%error, "MCP proxy disconnected with error");
                    }
                });
            }
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    let _ = std::fs::remove_file(&socket);
    Ok(())
}

impl Gateway {
    async fn subscribe(
        self: &Arc<Self>,
        subscription: &Subscription,
    ) -> Result<(InstanceKey, Arc<Bridge>, Scope), String> {
        let definition = self
            .servers
            .get(&subscription.server)
            .cloned()
            .ok_or_else(|| format!("unknown MCP server: {}", subscription.server))?;
        let workspace = match definition.scope {
            Scope::Global => None,
            Scope::Workspace => Some(subscription.workspace.clone()),
        };
        let key = InstanceKey {
            server: subscription.server.clone(),
            workspace: workspace.clone(),
        };
        let mut instances = self.instances.lock().await;
        let bridge = Arc::clone(
            instances
                .entry(key.clone())
                .or_insert_with(|| Arc::new(Bridge::new(definition.clone(), workspace))),
        );
        Ok((key, bridge, definition.scope))
    }

    fn schedule_idle_cleanup(
        self: &Arc<Self>,
        key: InstanceKey,
        bridge: Arc<Bridge>,
        generation: u64,
    ) {
        let gateway = Arc::clone(self);
        tokio::spawn(async move {
            sleep(bridge.idle_timeout()).await;
            if bridge.active_clients() != 0 || bridge.idle_generation() != generation {
                return;
            }
            let mut instances = gateway.instances.lock().await;
            if instances
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &bridge))
            {
                instances.remove(&key);
            }
        });
    }
}

async fn handle_client(stream: UnixStream, gateway: Arc<Gateway>) -> Result<(), ClientError> {
    let client_id = gateway.next_client_id.fetch_add(1, Ordering::Relaxed);
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let header = lines
        .next_line()
        .await?
        .ok_or(ClientError::MissingSubscription)?;
    let subscription: Subscription = serde_json::from_str(&header)?;
    let (key, bridge, scope) = match gateway.subscribe(&subscription).await {
        Ok(subscription) => subscription,
        Err(message) => {
            write_message(&mut writer, &rpc_error(Value::Null, -32000, &message)).await?;
            return Ok(());
        }
    };

    let (sender, mut responses) = mpsc::unbounded_channel::<Value>();
    let writer_task = tokio::spawn(async move {
        while let Some(response) = responses.recv().await {
            write_message(&mut writer, &response).await?;
        }
        Ok::<(), ClientError>(())
    });

    let mut protocol_version: Option<String> = None;
    while let Some(line) = lines.next_line().await? {
        let message = match serde_json::from_str::<Value>(&line) {
            Ok(message) => message,
            Err(error) => {
                let _ = sender.send(rpc_error(Value::Null, -32700, &error.to_string()));
                continue;
            }
        };
        let method = message.get("method").and_then(Value::as_str);
        if method == Some("initialize") {
            if protocol_version.is_some() {
                let id = message.get("id").cloned().unwrap_or(Value::Null);
                let _ = sender.send(rpc_error(id, -32600, "client already initialized"));
                continue;
            }
            match bridge.initialize(client_id, sender.clone(), &message).await {
                Ok((version, response, initialized)) => {
                    if initialized {
                        protocol_version = Some(version);
                    }
                    let _ = sender.send(response);
                }
                Err(error) => {
                    let id = message.get("id").cloned().unwrap_or(Value::Null);
                    let _ = sender.send(rpc_error(id, -32001, &error.to_string()));
                }
            }
            continue;
        }

        let Some(version) = &protocol_version else {
            if let Some(id) = message.get("id") {
                let _ = sender.send(rpc_error(id.clone(), -32002, "client not initialized"));
            }
            continue;
        };
        let request_id = message.get("id").cloned();
        if let Err(error) = bridge.forward(version, client_id, message).await {
            tracing::warn!(%error, client_id, "failed to forward MCP message");
            if let Some(id) = request_id {
                let _ = sender.send(rpc_error(id, -32003, &error.to_string()));
            }
        }
    }

    if let Some(version) = protocol_version {
        let became_idle = bridge.detach(&version, client_id).await;
        if became_idle && scope == Scope::Workspace {
            let generation = bridge.idle_generation();
            gateway.schedule_idle_cleanup(key, bridge, generation);
        }
    }
    drop(sender);
    writer_task.await.map_err(ClientError::WriterTask)??;
    Ok(())
}

async fn write_message(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    message: &Value,
) -> Result<(), ClientError> {
    let mut encoded = serde_json::to_vec(message)?;
    encoded.push(b'\n');
    writer.write_all(&encoded).await?;
    writer.flush().await?;
    Ok(())
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

async fn prepare_socket(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    match UnixStream::connect(path).await {
        Ok(_) => Err(std::io::Error::new(
            ErrorKind::AddrInUse,
            "another MCP gateway is already listening",
        )),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionRefused | ErrorKind::NotFound
            ) =>
        {
            std::fs::remove_file(path)
        }
        Err(error) => Err(error),
    }
}

fn default_socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("nyx-mcp-gateway.sock")
}

#[derive(Debug, thiserror::Error)]
enum ClientError {
    #[error("proxy disconnected before subscription")]
    MissingSubscription,
    #[error("proxy I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("proxy sent invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("proxy writer task failed: {0}")]
    WriterTask(tokio::task::JoinError),
}
