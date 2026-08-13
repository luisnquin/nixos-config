use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::process::Command;

use crate::discover::sweep;
use crate::registry::state_dir;

/// Where the multiplexed control sockets live. A survey fans out several
/// commands per host, and without a shared connection each one pays its own
/// handshake; `%C` keys the socket on the host/port/user tuple ssh resolved.
fn control_path() -> PathBuf {
    let dir = state_dir().join("ssh");
    let _ = std::fs::create_dir_all(&dir);

    dir.join("%C")
}

/// An ssh invocation carrying nothing but connection hygiene. Every fact about
/// where `host` is and how to authenticate to it stays in ssh's own config.
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

/// Wraps a string so the remote shell reads it as exactly one word.
fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// One remote command line running `script` with `args` bound to `$1`, `$2`, …
/// Passing values as positional arguments rather than interpolating them into
/// the script keeps a device id or bundle id from being read as shell syntax.
///
/// Quoted into a single word rather than passed as separate arguments because
/// ssh has no argv to pass: it joins everything after the host with spaces and
/// hands the result to the remote login shell, which splits it again on its own
/// rules. A multi-line script given as one argument comes apart on the way.
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

/// Runs `remote` and returns its stdout as trimmed text, or an empty string for
/// anything that failed. Discovery calls this per host and one unreachable
/// machine must not fail the survey.
pub async fn text(host: &str, remote: &str, limit: Duration) -> String {
    let mut cmd = command(host);
    cmd.arg(remote);

    match output(cmd, limit).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Brings up a forward from `local` to `remote_port` on the host's own
/// loopback, reusing one that is already up.
///
/// `-f -N` daemonises, so the tunnel outlives the `phone` process that opened
/// it. That is deliberate: every CLI invocation is a fresh process, and a
/// tunnel torn down on exit would be rebuilt on every single command.
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

/// A free local port, for a forward that has to be recorded before it exists.
/// The bind is released immediately, so this is a hint rather than a
/// reservation — callers keep the number and re-pick if it stops working.
pub async fn free_port() -> Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    drop(listener);

    Ok(port)
}
