use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::Parser;
use nyx_mcp_gateway::protocol::Subscription;

#[derive(Parser)]
struct Args {
    server: String,
    #[arg(long)]
    socket: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let socket = args.socket.unwrap_or_else(default_socket);
    let stream = UnixStream::connect(&socket)?;
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

fn default_socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("nyx-mcp-gateway.sock")
}
