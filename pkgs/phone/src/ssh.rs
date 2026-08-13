use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::process::Command;

use crate::discover::sweep;
use crate::registry::state_dir;

/// Multiplexed control sockets, so a survey's several commands per host share
/// one handshake. `%C` keys the socket on the tuple ssh resolved.
fn control_path() -> PathBuf {
    let dir = state_dir().join("ssh");
    let _ = std::fs::create_dir_all(&dir);

    dir.join("%C")
}

/// Connection hygiene only; where `host` is and how to reach it is ssh's.
pub fn command(host: &str) -> Command {
    let mut cmd = Command::new("ssh");

    cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5"]);
    cmd.args(["-o", "ControlMaster=auto", "-o", "ControlPersist=60"]);
    cmd.arg("-o")
        .arg(format!("ControlPath={}", control_path().display()));
    cmd.arg(host);
    cmd.stdin(Stdio::null());

    cmd
}

fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// One remote command line running `script` with `args` bound to `$1`, `$2`, …
/// Positional rather than interpolated, so a device or bundle id is never read
/// as shell syntax.
///
/// Quoted into a single word because ssh has no argv: it joins everything after
/// the host with spaces and lets the remote shell split it again.
pub fn remote(script: &str, args: &[&str]) -> String {
    let mut out = format!("sh -c {} sh", quoted(script));

    for arg in args {
        out.push(' ');
        out.push_str(&quoted(arg));
    }

    out
}

pub fn script(host: &str, script: &str, args: &[&str]) -> Command {
    let mut cmd = command(host);

    cmd.arg(remote(script, args));

    cmd
}

pub async fn output(mut cmd: Command, limit: Duration) -> Result<Vec<u8>> {
    let out = tokio::time::timeout(limit, cmd.stderr(Stdio::null()).output())
        .await
        .map_err(|_| anyhow!("ssh timed out"))??;

    Ok(out.stdout)
}

/// Empty string for anything that failed: discovery calls this per host and one
/// unreachable machine must not fail the survey.
pub async fn text(host: &str, remote: &str, limit: Duration) -> String {
    let mut cmd = command(host);
    cmd.arg(remote);

    match output(cmd, limit).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// A forward from `local` to `remote_port` on the host's loopback, reusing one
/// already up. `-f -N` daemonises so it outlives this process — every CLI
/// invocation is a fresh one, and a tunnel torn down on exit is rebuilt on every
/// command.
pub async fn forward(host: &str, local: u16, remote_port: u16) -> Result<()> {
    if sweep::probe("127.0.0.1", local, Duration::from_millis(400)).await {
        return Ok(());
    }

    let status = tokio::time::timeout(
        Duration::from_secs(15),
        command(host)
            .args(["-o", "ExitOnForwardFailure=yes"])
            .args(["-f", "-N", "-L"])
            .arg(format!("127.0.0.1:{local}:127.0.0.1:{remote_port}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await
    .map_err(|_| anyhow!("{host}: forward timed out"))??;

    if !status.success() {
        bail!("{host}: could not forward port {remote_port}");
    }

    Ok(())
}

/// A hint rather than a reservation: the bind is released immediately, so
/// callers keep the number and re-pick if it stops working.
pub async fn free_port() -> Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    drop(listener);

    Ok(port)
}
