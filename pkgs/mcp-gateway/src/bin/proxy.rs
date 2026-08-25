use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use nyx_mcp_gateway::protocol::Subscription;
use tokio::io::{AsyncWriteExt, copy};

#[derive(Parser)]
struct Args {
    server: String,
    #[arg(long)]
    socket: Option<PathBuf>,
    #[arg(long, default_value_t = 4)]
    connect_timeout_sec: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let socket = args.socket.unwrap_or_else(default_socket);
    let stream = connect_with_retry(&socket, Duration::from_secs(args.connect_timeout_sec)).await?;
    let (mut daemon_reader, mut daemon_writer) = stream.into_split();
    let subscription = Subscription {
        server: args.server,
        workspace: std::fs::canonicalize(std::env::current_dir()?)?,
    };
    let mut encoded = serde_json::to_vec(&subscription)?;
    encoded.push(b'\n');
    daemon_writer.write_all(&encoded).await?;
    daemon_writer.flush().await?;

    let input = async {
        copy(&mut tokio::io::stdin(), &mut daemon_writer).await?;
        daemon_writer.shutdown().await
    };
    let mut stdout = tokio::io::stdout();
    let output = copy(&mut daemon_reader, &mut stdout);
    tokio::try_join!(input, output)?;
    Ok(())
}

async fn connect_with_retry(
    socket: &std::path::Path,
    timeout: Duration,
) -> std::io::Result<tokio::net::UnixStream> {
    let deadline = Instant::now() + timeout;

    loop {
        match tokio::net::UnixStream::connect(socket).await {
            Ok(stream) => return Ok(stream),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::Interrupted
                ) && Instant::now() < deadline =>
            {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

fn default_socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("/run/user").join(rustix::process::geteuid().as_raw().to_string())
        })
        .join("nyx-mcp-gateway.sock")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::connect_with_retry;

    #[tokio::test]
    async fn waits_for_daemon_socket() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "nyx-mcp-gw-client-test-{}-{suffix}",
            std::process::id()
        ));
        let socket = directory.join("gateway.sock");
        std::fs::create_dir(&directory).expect("create test directory");

        let listener_socket = socket.clone();
        let listener = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            tokio::net::UnixListener::bind(listener_socket).expect("bind delayed socket")
        });

        let stream = connect_with_retry(&socket, Duration::from_secs(1)).await;
        assert!(stream.is_ok());

        drop(listener.await.expect("listener task panicked"));
        std::fs::remove_dir_all(directory).expect("remove test directory");
    }
}
