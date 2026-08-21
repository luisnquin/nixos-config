use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use nyx_mcp_gateway::protocol::Subscription;

#[derive(Parser)]
struct Args {
    server: String,
    #[arg(long)]
    socket: Option<PathBuf>,
    #[arg(long, default_value_t = 4)]
    connect_timeout_sec: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let socket = args.socket.unwrap_or_else(default_socket);
    let stream = connect_with_retry(&socket, Duration::from_secs(args.connect_timeout_sec))?;
    let mut daemon_writer = stream.try_clone()?;
    let mut daemon_reader = stream;
    let subscription = Subscription {
        server: args.server,
        workspace: std::fs::canonicalize(std::env::current_dir()?)?,
    };
    serde_json::to_writer(&mut daemon_writer, &subscription)?;
    daemon_writer.write_all(b"\n")?;
    daemon_writer.flush()?;

    let input = std::thread::spawn(move || -> std::io::Result<()> {
        let stdin = std::io::stdin();
        std::io::copy(&mut stdin.lock(), &mut daemon_writer)?;
        daemon_writer.shutdown(Shutdown::Write)?;
        Ok(())
    });
    let output = std::thread::spawn(move || -> std::io::Result<()> {
        let stdout = std::io::stdout();
        std::io::copy(&mut daemon_reader, &mut stdout.lock())?;
        Ok(())
    });

    let output_result = output.join().map_err(|_| "proxy output thread panicked")?;
    output_result?;
    drop(input);
    Ok(())
}

fn connect_with_retry(socket: &std::path::Path, timeout: Duration) -> std::io::Result<UnixStream> {
    let deadline = Instant::now() + timeout;

    loop {
        match UnixStream::connect(socket) {
            Ok(stream) => return Ok(stream),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::Interrupted
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
}

fn default_socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("nyx-mcp-gateway.sock")
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::connect_with_retry;

    #[test]
    fn waits_for_daemon_socket() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before Unix epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "nyx-mcp-proxy-test-{}-{suffix}",
            std::process::id()
        ));
        let socket = directory.join("gateway.sock");
        std::fs::create_dir(&directory).expect("create test directory");

        let listener_socket = socket.clone();
        let listener = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            UnixListener::bind(listener_socket).expect("bind delayed socket")
        });

        let stream = connect_with_retry(&socket, Duration::from_secs(1));
        assert!(stream.is_ok());

        drop(listener.join().expect("listener thread panicked"));
        std::fs::remove_dir_all(directory).expect("remove test directory");
    }
}
