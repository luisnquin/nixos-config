use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};

use crate::compat::{self, CompatError, Version};
use crate::config::{NotificationPolicy, Server};

pub type ClientId = u64;
pub type ClientSender = mpsc::UnboundedSender<Value>;

/// One shared upstream process, serving every downstream client regardless of its protocol
/// revision. The canonical revision is spoken upstream; each client is answered in its own.
pub struct Bridge {
    runtime: Arc<Runtime>,
    active_clients: AtomicUsize,
    idle_generation: AtomicU64,
}

#[derive(Clone)]
struct Registration {
    sender: ClientSender,
    version: Version,
}

struct Runtime {
    server: String,
    definition: Server,
    workspace: Option<PathBuf>,
    canonical: Version,
    state: Mutex<RuntimeState>,
    clients: Arc<Mutex<HashMap<ClientId, Registration>>>,
    next_request_id: AtomicU64,
}

#[derive(Default)]
struct RuntimeState {
    connection: Option<Arc<Connection>>,
    initialize_outcome: Option<Value>,
    upstream_version: Option<Version>,
}

struct Connection {
    writer: Arc<Mutex<ChildStdin>>,
    routing: Arc<Routing>,
    dead: Arc<AtomicBool>,
    started: Instant,
    last_activity_ms: Arc<AtomicU64>,
    reader_task: JoinHandle<()>,
    process_task: JoinHandle<()>,
    reaper_task: Option<JoinHandle<()>>,
}

struct Routing {
    server: String,
    canonical: Version,
    negotiated: OnceLock<Version>,
    pending: Mutex<HashMap<u64, PendingRequest>>,
    request_ids: Mutex<HashMap<RequestKey, u64>>,
    clients: Arc<Mutex<HashMap<ClientId, Registration>>>,
    writer: Arc<Mutex<ChildStdin>>,
    policy: NotificationPolicy,
}

enum PendingRequest {
    Internal(oneshot::Sender<Value>),
    Client {
        client_id: ClientId,
        sender: ClientSender,
        original_id: Value,
        progress_token: Option<Value>,
        method: String,
        version: Version,
    },
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct RequestKey {
    client_id: ClientId,
    id: String,
}

impl Bridge {
    #[must_use]
    pub fn new(
        server: String,
        definition: Server,
        workspace: Option<PathBuf>,
        canonical: Version,
    ) -> Self {
        Self {
            runtime: Arc::new(Runtime {
                server,
                definition,
                workspace,
                canonical,
                state: Mutex::new(RuntimeState::default()),
                clients: Arc::new(Mutex::new(HashMap::new())),
                next_request_id: AtomicU64::new(1),
            }),
            active_clients: AtomicUsize::new(0),
            idle_generation: AtomicU64::new(0),
        }
    }

    /// Serves a downstream `initialize` from the single shared upstream handshake.
    ///
    /// The client is answered with the revision it requested; the upstream link stays on the
    /// canonical revision no matter how many revisions the connected clients span.
    ///
    /// # Errors
    ///
    /// Fails when the client requests an unsupported revision, when the upstream server refuses
    /// the canonical revision, or when the upstream process cannot be started.
    pub async fn initialize(
        &self,
        client_id: ClientId,
        sender: ClientSender,
        request: &Value,
    ) -> Result<(Version, Value, bool), BridgeError> {
        let original_id = request.get("id").cloned().unwrap_or(Value::Null);
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let requested = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .ok_or(BridgeError::MissingProtocolVersion)?;
        let version = Version::parse(requested).map_err(|error| {
            tracing::error!(
                mcp.event = "protocol_rejected",
                mcp.server = %self.runtime.server,
                mcp.client_id = client_id,
                mcp.requested_version = requested,
                mcp.supported_versions = %compat::SUPPORTED_VERSIONS.join(", "),
                mcp.reason = %error,
                "refusing MCP client on an unsupported protocol revision"
            );
            error
        })?;

        let (outcome, upstream) = self.runtime.initialize().await?;
        if outcome.get("error").is_some() {
            return Ok((version, restore_id(outcome, original_id), false));
        }

        if version == upstream {
            tracing::debug!(
                mcp.event = "protocol_match",
                mcp.server = %self.runtime.server,
                mcp.client_id = client_id,
                mcp.version = version.as_str(),
                "MCP client matches the shared upstream revision"
            );
        } else {
            tracing::info!(
                mcp.event = "protocol_bridged",
                mcp.server = %self.runtime.server,
                mcp.client_id = client_id,
                mcp.client_version = version.as_str(),
                mcp.upstream_version = upstream.as_str(),
                "MCP client and shared upstream speak different revisions; translating in place"
            );
        }

        let mut result = outcome;
        translate_for_client(
            &self.runtime.server,
            client_id,
            "initialize",
            upstream,
            version,
            &mut result,
        )?;
        if let Some(object) = result.get_mut("result").and_then(Value::as_object_mut) {
            object.insert(
                "protocolVersion".to_owned(),
                Value::String(version.as_str().to_owned()),
            );
        }

        self.runtime
            .clients
            .lock()
            .await
            .insert(client_id, Registration { sender, version });
        self.active_clients.fetch_add(1, Ordering::AcqRel);
        Ok((version, restore_id(result, original_id), true))
    }

    /// Forwards a downstream request or notification to the shared upstream process.
    ///
    /// # Errors
    ///
    /// Fails when the request carries semantics the canonical revision does not define, when the
    /// client is unknown, or when the upstream link is broken.
    pub async fn forward(&self, client_id: ClientId, message: Value) -> Result<(), BridgeError> {
        self.runtime.forward(client_id, message).await
    }

    pub async fn detach(&self, client_id: ClientId) -> bool {
        self.runtime.detach(client_id).await;
        let previous = self.active_clients.fetch_sub(1, Ordering::AcqRel);
        if previous == 1 {
            self.idle_generation.fetch_add(1, Ordering::AcqRel);
            return true;
        }
        false
    }

    #[must_use]
    pub fn active_clients(&self) -> usize {
        self.active_clients.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn idle_generation(&self) -> u64 {
        self.idle_generation.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.runtime.definition.idle_seconds)
    }
}

impl Runtime {
    async fn initialize(&self) -> Result<(Value, Version), BridgeError> {
        let mut state = self.state.lock().await;
        let upstream = state.upstream_version.unwrap_or(self.canonical);
        if let (Some(connection), Some(outcome)) = (&state.connection, &state.initialize_outcome)
            && !connection.is_dead()
        {
            return Ok((outcome.clone(), upstream));
        }

        // Re-checked ahead of the reaped-process branch so that a workspace gaining the file
        // still reaches the spawn below rather than replaying a cached empty surface.
        if !self.precondition_met() {
            tracing::info!(
                mcp.event = "precondition_unmet",
                mcp.server = %self.server,
                mcp.workspace = ?self.workspace,
                "serving an empty MCP surface; no precondition file exists in the workspace"
            );
            let outcome = synthetic_initialize(self.canonical);
            // Retained so a client attached under the empty surface can still reach the spawn
            // path once the file appears, instead of failing as never-initialized.
            state.initialize_outcome = Some(outcome.clone());
            return Ok((outcome, self.canonical));
        }

        // A reaped process revives on the next tool call, not on every new client handshake.
        if self.definition.reap_when_idle
            && let Some(outcome) = &state.initialize_outcome
        {
            return Ok((outcome.clone(), upstream));
        }

        let (connection, outcome) = self.spawn_initialized().await?;
        let upstream = connection.upstream();
        state.connection = Some(connection);
        state.initialize_outcome = Some(outcome.clone());
        state.upstream_version = Some(upstream);
        Ok((outcome, upstream))
    }

    fn precondition_met(&self) -> bool {
        let Some(requires) = &self.definition.requires else {
            return true;
        };
        let Some(workspace) = self.workspace.as_deref() else {
            return false;
        };
        requires
            .any_file_exists
            .iter()
            .any(|file| workspace.join(file).exists())
    }

    async fn forward(&self, client_id: ClientId, message: Value) -> Result<(), BridgeError> {
        let connected = {
            let state = self.state.lock().await;
            state
                .connection
                .as_ref()
                .is_some_and(|connection| !connection.is_dead())
        };
        if !connected && !self.precondition_met() {
            return self.forward_synthetic(client_id, message).await;
        }

        let connection = self.connection().await?;
        connection.touch();
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if method == "notifications/cancelled" {
            return connection.cancel(client_id, &message).await;
        }

        let registration = self
            .clients
            .lock()
            .await
            .get(&client_id)
            .cloned()
            .ok_or(BridgeError::UnknownClient)?;

        compat::adapt_request(
            &method,
            registration.version,
            connection.upstream(),
            &message,
        )
        .map_err(|error| {
            tracing::warn!(
                mcp.event = "contract_rejected",
                mcp.server = %self.server,
                mcp.client_id = client_id,
                mcp.method = %method,
                mcp.client_version = registration.version.as_str(),
                mcp.upstream_version = connection.upstream().as_str(),
                mcp.reason = %error,
                "refusing an MCP request the upstream revision cannot represent"
            );
            error
        })?;

        if message.get("id").is_none() {
            return connection.notify(&message).await;
        }

        let internal_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        connection
            .request_client(
                client_id,
                registration.sender,
                registration.version,
                method,
                internal_id,
                message,
                self.timeout(),
            )
            .await
    }

    /// Answers in place of a process the precondition forbids spawning: an empty tool list
    /// instead of an error, so clients see a server with nothing to offer.
    async fn forward_synthetic(
        &self,
        client_id: ClientId,
        message: Value,
    ) -> Result<(), BridgeError> {
        let Some(original_id) = message.get("id").cloned() else {
            return Ok(());
        };
        let registration = self
            .clients
            .lock()
            .await
            .get(&client_id)
            .cloned()
            .ok_or(BridgeError::UnknownClient)?;
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let response = match method {
            "ping" => json!({"jsonrpc": "2.0", "id": original_id, "result": {}}),
            "tools/list" => json!({"jsonrpc": "2.0", "id": original_id, "result": {"tools": []}}),
            "prompts/list" => {
                json!({"jsonrpc": "2.0", "id": original_id, "result": {"prompts": []}})
            }
            "resources/list" => {
                json!({"jsonrpc": "2.0", "id": original_id, "result": {"resources": []}})
            }
            _ => rpc_error(
                original_id,
                -32601,
                "MCP server precondition unmet in this workspace",
            ),
        };
        let _ = registration.sender.send(response);
        Ok(())
    }

    async fn detach(&self, client_id: ClientId) {
        self.clients.lock().await.remove(&client_id);
        let connection = self.state.lock().await.connection.clone();
        if let Some(connection) = connection {
            connection.detach(client_id).await;
        }
    }

    async fn connection(&self) -> Result<Arc<Connection>, BridgeError> {
        let mut state = self.state.lock().await;
        if let Some(connection) = &state.connection
            && !connection.is_dead()
        {
            return Ok(Arc::clone(connection));
        }
        if state.initialize_outcome.is_none() {
            return Err(BridgeError::NotInitialized);
        }
        let (connection, outcome) = self.spawn_initialized().await?;
        if outcome.get("error").is_some() {
            return Err(BridgeError::InitializationRejected);
        }
        state.connection = Some(Arc::clone(&connection));
        state.initialize_outcome = Some(outcome);
        state.upstream_version = Some(connection.upstream());
        Ok(connection)
    }

    async fn spawn_initialized(&self) -> Result<(Arc<Connection>, Value), BridgeError> {
        let connection = Arc::new(
            Connection::spawn(
                &self.server,
                self.canonical,
                &self.definition,
                self.workspace.as_deref(),
                Arc::clone(&self.clients),
            )
            .await?,
        );
        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": self.canonical.as_str(),
                "capabilities": {},
                "clientInfo": {
                    "name": "nyx-mcp-gateway",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            },
        });
        let response = connection
            .request_internal(&request, self.timeout())
            .await?;
        if response.get("error").is_none() {
            let negotiated = response
                .pointer("/result/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if negotiated != self.canonical.as_str() {
                match Version::parse_downlevel(negotiated) {
                    Some(version) if version < self.canonical => {
                        let _ = connection.routing.negotiated.set(version);
                        tracing::info!(
                            mcp.event = "protocol_downlevel",
                            mcp.server = %self.server,
                            mcp.negotiated_version = negotiated,
                            mcp.canonical_version = self.canonical.as_str(),
                            "upstream MCP server negotiated an older revision; \
                             its results need no rewriting for newer clients"
                        );
                    }
                    _ => {
                        let error = CompatError::UpstreamVersionMismatch {
                            negotiated: negotiated.to_owned(),
                            canonical: self.canonical.as_str(),
                        };
                        tracing::error!(
                            mcp.event = "protocol_rejected",
                            mcp.server = %self.server,
                            mcp.negotiated_version = negotiated,
                            mcp.canonical_version = self.canonical.as_str(),
                            mcp.reason = %error,
                            "upstream MCP server refused the canonical revision"
                        );
                        return Err(BridgeError::Compat(error));
                    }
                }
            }
            tracing::info!(
                mcp.event = "upstream_started",
                mcp.server = %self.server,
                mcp.upstream_version = connection.upstream().as_str(),
                mcp.workspace = ?self.workspace,
                "started one shared MCP upstream process"
            );
            connection
                .notify(&json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                }))
                .await?;
        }
        Ok((connection, strip_id(response)))
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.definition.timeout_seconds)
    }
}

fn translate_for_client(
    server: &str,
    client_id: ClientId,
    method: &str,
    upstream: Version,
    client: Version,
    message: &mut Value,
) -> Result<(), CompatError> {
    let Some(result) = message.get_mut("result") else {
        return Ok(());
    };
    let notes = compat::downgrade_result(method, upstream, client, result)?;
    for note in notes {
        tracing::info!(
            mcp.event = "payload_translated",
            mcp.server = server,
            mcp.client_id = client_id,
            mcp.method = method,
            mcp.field = note.field,
            mcp.path = %note.path,
            mcp.upstream_version = upstream.as_str(),
            mcp.client_version = client.as_str(),
            mcp.detail = note.detail,
            "translated an MCP payload across protocol revisions"
        );
    }
    Ok(())
}

impl Connection {
    async fn spawn(
        server: &str,
        canonical: Version,
        definition: &Server,
        workspace: Option<&std::path::Path>,
        clients: Arc<Mutex<HashMap<ClientId, Registration>>>,
    ) -> Result<Self, BridgeError> {
        let mut command = Command::new(&definition.command);
        command
            .args(&definition.args)
            .envs(&definition.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        for (name, path) in &definition.credentials {
            let value = std::fs::read_to_string(path)
                .map_err(|source| BridgeError::Credential {
                    path: path.clone(),
                    source,
                })?
                .trim_end_matches(['\r', '\n'])
                .to_owned();
            command.env(name, value);
        }
        if let Some(workspace) = workspace {
            command.current_dir(workspace);
        }

        let mut child = command.spawn().map_err(BridgeError::Spawn)?;
        let stdin = child.stdin.take().ok_or(BridgeError::MissingPipe)?;
        let stdout = child.stdout.take().ok_or(BridgeError::MissingPipe)?;
        let writer = Arc::new(Mutex::new(stdin));
        let routing = Arc::new(Routing {
            server: server.to_owned(),
            canonical,
            negotiated: OnceLock::new(),
            pending: Mutex::new(HashMap::new()),
            request_ids: Mutex::new(HashMap::new()),
            clients,
            writer: Arc::clone(&writer),
            policy: definition.notifications,
        });
        let dead = Arc::new(AtomicBool::new(false));

        let reader_routing = Arc::clone(&routing);
        let reader_dead = Arc::clone(&dead);
        let reader_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => reader_routing.route(&line).await,
                    Ok(None) => break,
                    Err(error) => {
                        tracing::warn!(%error, "MCP server stdout failed");
                        break;
                    }
                }
            }
            reader_dead.store(true, Ordering::Release);
            reader_routing.fail_all("MCP server exited").await;
        });

        let started = Instant::now();
        let last_activity_ms = Arc::new(AtomicU64::new(0));
        let (kill_sender, kill_receiver) = oneshot::channel::<()>();

        let process_dead = Arc::clone(&dead);
        let process_task = tokio::spawn(async move {
            tokio::select! {
                status = child.wait() => match status {
                    Ok(status) => tracing::warn!(%status, "MCP server exited"),
                    Err(error) => tracing::warn!(%error, "failed to wait for MCP server"),
                },
                // A dropped sender disables this branch, leaving the wait in place.
                Ok(()) = kill_receiver => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }
            }
            process_dead.store(true, Ordering::Release);
        });

        let reaper_task = definition.reap_when_idle.then(|| {
            let idle = Duration::from_secs(definition.idle_seconds);
            let server = server.to_owned();
            let dead = Arc::clone(&dead);
            let last_activity_ms = Arc::clone(&last_activity_ms);
            let routing = Arc::clone(&routing);
            tokio::spawn(async move {
                let mut kill_sender = Some(kill_sender);
                loop {
                    let idle_for = Duration::from_millis(
                        elapsed_ms(started)
                            .saturating_sub(last_activity_ms.load(Ordering::Acquire)),
                    );
                    if idle_for < idle {
                        sleep(idle - idle_for).await;
                        continue;
                    }
                    if dead.load(Ordering::Acquire) {
                        return;
                    }
                    if !routing.pending.lock().await.is_empty() {
                        sleep(idle).await;
                        continue;
                    }
                    dead.store(true, Ordering::Release);
                    if let Some(sender) = kill_sender.take() {
                        let _ = sender.send(());
                    }
                    tracing::info!(
                        mcp.event = "upstream_reaped",
                        mcp.server = %server,
                        mcp.idle_seconds = idle.as_secs(),
                        "stopped an idle MCP upstream process; the next call respawns it"
                    );
                    return;
                }
            })
        });

        Ok(Self {
            writer,
            routing,
            dead,
            started,
            last_activity_ms,
            reader_task,
            process_task,
            reaper_task,
        })
    }

    fn touch(&self) {
        self.last_activity_ms
            .store(elapsed_ms(self.started), Ordering::Release);
    }

    fn upstream(&self) -> Version {
        self.routing.upstream()
    }

    async fn request_internal(
        &self,
        message: &Value,
        duration: Duration,
    ) -> Result<Value, BridgeError> {
        let (sender, receiver) = oneshot::channel();
        self.routing
            .pending
            .lock()
            .await
            .insert(0, PendingRequest::Internal(sender));
        self.write(message).await?;
        timeout(duration, receiver)
            .await
            .map_err(|_| BridgeError::Timeout)?
            .map_err(|_| BridgeError::Exited)
    }

    #[allow(clippy::too_many_arguments)]
    async fn request_client(
        &self,
        client_id: ClientId,
        sender: ClientSender,
        version: Version,
        method: String,
        internal_id: u64,
        mut message: Value,
        duration: Duration,
    ) -> Result<(), BridgeError> {
        let original_id = message.get("id").cloned().unwrap_or(Value::Null);
        let request_key = RequestKey {
            client_id,
            id: id_key(&original_id),
        };
        message["id"] = Value::from(internal_id);
        message["params"]["_meta"]["io.nyx/session-id"] = Value::String(client_id.to_string());
        let progress_token = take_progress_token(&mut message);
        self.routing.pending.lock().await.insert(
            internal_id,
            PendingRequest::Client {
                client_id,
                sender,
                original_id,
                progress_token,
                method,
                version,
            },
        );
        self.routing
            .request_ids
            .lock()
            .await
            .insert(request_key, internal_id);

        if let Err(error) = self.write(&message).await {
            self.routing.remove_pending(internal_id).await;
            return Err(error);
        }

        let routing = Arc::clone(&self.routing);
        tokio::spawn(async move {
            sleep(duration).await;
            if let Some(PendingRequest::Client {
                sender,
                original_id,
                ..
            }) = routing.remove_pending(internal_id).await
            {
                let _ = sender.send(rpc_error(
                    original_id,
                    -32003,
                    "MCP server request timed out",
                ));
                let _ = write_json(
                    &routing.writer,
                    &json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/cancelled",
                        "params": { "requestId": internal_id },
                    }),
                )
                .await;
            }
        });
        Ok(())
    }

    async fn cancel(&self, client_id: ClientId, message: &Value) -> Result<(), BridgeError> {
        let Some(original_id) = message.pointer("/params/requestId") else {
            return Ok(());
        };
        let key = RequestKey {
            client_id,
            id: id_key(original_id),
        };
        let internal_id = self.routing.request_ids.lock().await.get(&key).copied();
        if let Some(internal_id) = internal_id {
            self.notify(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancelled",
                "params": { "requestId": internal_id },
            }))
            .await?;
        }
        Ok(())
    }

    async fn detach(&self, client_id: ClientId) {
        let ids: Vec<u64> = {
            let pending = self.routing.pending.lock().await;
            pending
                .iter()
                .filter_map(|(id, request)| match request {
                    PendingRequest::Client {
                        client_id: owner, ..
                    } if *owner == client_id => Some(*id),
                    _ => None,
                })
                .collect()
        };
        for id in ids {
            self.routing.remove_pending(id).await;
            let _ = self
                .notify(&json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/cancelled",
                    "params": { "requestId": id },
                }))
                .await;
        }
    }

    async fn notify(&self, message: &Value) -> Result<(), BridgeError> {
        self.write(message).await
    }

    async fn write(&self, message: &Value) -> Result<(), BridgeError> {
        if self.is_dead() {
            return Err(BridgeError::Exited);
        }
        write_json(&self.writer, message).await
    }

    fn is_dead(&self) -> bool {
        self.dead.load(Ordering::Acquire)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.reader_task.abort();
        self.process_task.abort();
        if let Some(reaper_task) = &self.reaper_task {
            reaper_task.abort();
        }
    }
}

impl Routing {
    fn upstream(&self) -> Version {
        self.negotiated.get().copied().unwrap_or(self.canonical)
    }

    async fn route(&self, line: &str) {
        let Ok(mut message) = serde_json::from_str::<Value>(line) else {
            tracing::warn!("ignoring non-JSON MCP server output");
            return;
        };

        if message.get("method").is_none() {
            self.route_response(message).await;
            return;
        }
        if message.get("id").is_some() {
            // Ping is bidirectional in every MCP revision and needs no client capability, so the
            // gateway answers it itself; encore uses it as a link keepalive.
            if message.get("method").and_then(Value::as_str) == Some("ping") {
                self.answer_server_ping(&message).await;
            } else {
                self.reject_server_request(&message).await;
            }
            return;
        }
        if message.get("method").and_then(Value::as_str) == Some("notifications/progress") {
            self.route_progress(&mut message).await;
            return;
        }
        if self.policy == NotificationPolicy::Broadcast {
            for registration in self.clients.lock().await.values() {
                let _ = registration.sender.send(message.clone());
            }
        }
    }

    async fn route_response(&self, mut message: Value) {
        let Some(internal_id) = message.get("id").and_then(Value::as_u64) else {
            return;
        };
        let Some(pending) = self.remove_pending(internal_id).await else {
            return;
        };
        match pending {
            PendingRequest::Internal(sender) => {
                let _ = sender.send(message);
            }
            PendingRequest::Client {
                client_id,
                sender,
                original_id,
                method,
                version,
                ..
            } => {
                if let Err(error) = translate_for_client(
                    &self.server,
                    client_id,
                    &method,
                    self.upstream(),
                    version,
                    &mut message,
                ) {
                    tracing::warn!(
                        mcp.event = "contract_rejected",
                        mcp.server = %self.server,
                        mcp.client_id = client_id,
                        mcp.method = %method,
                        mcp.client_version = version.as_str(),
                        mcp.upstream_version = self.upstream().as_str(),
                        mcp.reason = %error,
                        "refusing to deliver an MCP result that this client's revision cannot represent"
                    );
                    let _ = sender.send(rpc_error(original_id, -32004, &error.to_string()));
                    return;
                }
                message["id"] = original_id;
                let _ = sender.send(message);
            }
        }
    }

    async fn route_progress(&self, message: &mut Value) {
        let Some(internal_id) = message
            .pointer("/params/progressToken")
            .and_then(Value::as_str)
            .and_then(|token| token.strip_prefix("nyx:"))
            .and_then(|id| id.parse::<u64>().ok())
        else {
            return;
        };
        let pending = self.pending.lock().await;
        let Some(PendingRequest::Client {
            sender,
            progress_token: Some(original),
            ..
        }) = pending.get(&internal_id)
        else {
            return;
        };
        message["params"]["progressToken"] = original.clone();
        let _ = sender.send(message.clone());
    }

    async fn answer_server_ping(&self, message: &Value) {
        let response = json!({
            "jsonrpc": "2.0",
            "id": message.get("id").cloned().unwrap_or(Value::Null),
            "result": {},
        });
        if let Err(error) = write_json(&self.writer, &response).await {
            tracing::warn!(%error, "failed to answer a server-to-client ping");
        }
    }

    async fn reject_server_request(&self, message: &Value) {
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        tracing::warn!(
            mcp.event = "contract_rejected",
            mcp.server = %self.server,
            mcp.method = method,
            mcp.upstream_version = self.upstream().as_str(),
            mcp.reason = "the gateway advertises no client capabilities upstream",
            "refusing a server-to-client MCP request"
        );
        let response = rpc_error(
            message.get("id").cloned().unwrap_or(Value::Null),
            -32601,
            "gateway exposes no client capabilities",
        );
        if let Err(error) = write_json(&self.writer, &response).await {
            tracing::warn!(%error, "failed to reject server-to-client request");
        }
    }

    async fn remove_pending(&self, internal_id: u64) -> Option<PendingRequest> {
        let pending = self.pending.lock().await.remove(&internal_id);
        if let Some(PendingRequest::Client {
            client_id,
            original_id,
            ..
        }) = &pending
        {
            self.request_ids.lock().await.remove(&RequestKey {
                client_id: *client_id,
                id: id_key(original_id),
            });
        }
        pending
    }

    async fn fail_all(&self, message: &str) {
        let pending = std::mem::take(&mut *self.pending.lock().await);
        self.request_ids.lock().await.clear();
        for request in pending.into_values() {
            match request {
                PendingRequest::Internal(sender) => {
                    let _ = sender.send(rpc_error(Value::Null, -32001, message));
                }
                PendingRequest::Client {
                    sender,
                    original_id,
                    ..
                } => {
                    let _ = sender.send(rpc_error(original_id, -32001, message));
                }
            }
        }
    }
}

async fn write_json(writer: &Mutex<ChildStdin>, message: &Value) -> Result<(), BridgeError> {
    let mut encoded = serde_json::to_vec(message).map_err(BridgeError::Serialize)?;
    encoded.push(b'\n');
    let mut writer = writer.lock().await;
    writer
        .write_all(&encoded)
        .await
        .map_err(BridgeError::Write)?;
    writer.flush().await.map_err(BridgeError::Write)
}

fn take_progress_token(message: &mut Value) -> Option<Value> {
    let token = message.pointer("/params/_meta/progressToken").cloned();
    if token.is_some() {
        let id = message.get("id").and_then(Value::as_u64).unwrap_or(0);
        message["params"]["_meta"]["progressToken"] = Value::String(format!("nyx:{id}"));
    }
    token
}

fn id_key(id: &Value) -> String {
    serde_json::to_string(id).unwrap_or_else(|_| "null".to_owned())
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Handshake served for a server whose precondition files are absent: empty capabilities,
/// no process behind it.
fn synthetic_initialize(canonical: Version) -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": {
            "protocolVersion": canonical.as_str(),
            "capabilities": {},
            "serverInfo": {
                "name": "nyx-mcp-gateway",
                "version": env!("CARGO_PKG_VERSION"),
            },
        },
    })
}

fn strip_id(mut response: Value) -> Value {
    if let Some(object) = response.as_object_mut() {
        object.remove("id");
    }
    response
}

fn restore_id(mut response: Value, id: Value) -> Value {
    response["id"] = id;
    response
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("initialize request lacks protocolVersion")]
    MissingProtocolVersion,
    #[error("MCP server has not been initialized")]
    NotInitialized,
    #[error("MCP server rejected gateway reinitialization")]
    InitializationRejected,
    #[error("unknown gateway client")]
    UnknownClient,
    #[error("cannot start MCP server: {0}")]
    Spawn(std::io::Error),
    #[error("cannot read MCP credential {path}: {source}")]
    Credential {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("MCP server did not expose stdio")]
    MissingPipe,
    #[error("cannot encode MCP message: {0}")]
    Serialize(serde_json::Error),
    #[error("cannot write to MCP server: {0}")]
    Write(std::io::Error),
    #[error("MCP server exited")]
    Exited,
    #[error("MCP server request timed out")]
    Timeout,
    #[error("{0}")]
    Compat(#[from] CompatError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tokens_are_namespaced() {
        let mut request = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": { "_meta": { "progressToken": "client-token" } },
        });
        let token = take_progress_token(&mut request);
        assert_eq!(token, Some(json!("client-token")));
        assert_eq!(request["params"]["_meta"]["progressToken"], json!("nyx:42"));
    }

    #[test]
    fn response_ids_are_restored() {
        let response = strip_id(json!({"jsonrpc": "2.0", "id": 7, "result": {}}));
        assert_eq!(
            restore_id(response, json!("client-id"))["id"],
            json!("client-id")
        );
    }

    #[test]
    fn translation_rewrites_only_the_result_envelope() {
        let mut message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"tools": [{"name": "a", "title": "A"}]},
        });
        translate_for_client(
            "srv",
            1,
            "tools/list",
            Version::V20250618,
            Version::V20250326,
            &mut message,
        )
        .unwrap();
        assert_eq!(message["result"]["tools"][0].get("title"), None);
        assert_eq!(message["id"], json!(1));
    }

    #[test]
    fn error_envelopes_are_left_alone() {
        let mut message = json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -1, "message": "x"}});
        let before = message.clone();
        translate_for_client(
            "srv",
            1,
            "tools/call",
            Version::V20250618,
            Version::V20250326,
            &mut message,
        )
        .unwrap();
        assert_eq!(message, before);
    }
}
