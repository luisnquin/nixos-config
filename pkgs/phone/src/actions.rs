use std::io::Write as _;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};

use crate::adb;
use crate::connect::{attached_serial, Reporter};
use crate::ios;
use crate::model::{Device, Platform};

const PNG_MAGIC: [u8; 4] = [0x89, b'P', b'N', b'G'];

#[derive(Clone, Debug)]
pub enum Sink {
    Clipboard,
    Stdout,
    File(String),
}

impl Sink {
    pub fn from_opt(out: Option<&str>) -> Self {
        match out {
            None => Sink::Clipboard,
            Some("-") => Sink::Stdout,
            Some(path) => Sink::File(path.to_string()),
        }
    }
}

/// `screencap` and pymobiledevice3 both exit 0 on some failures, so a capture
/// only counts once real PNG bytes come back.
pub async fn screenshot(device: &Device, sink: &Sink, rep: &Reporter) -> Result<String> {
    let png = match device.platform {
        Platform::Ios => {
            let mut png = ios::screenshot(&device.id).await?;

            if !is_png(&png) {
                // tunneld may still be coming up right after a mac reboot
                rep.note("no image yet, retrying the tunnel");
                tokio::time::sleep(Duration::from_secs(3)).await;
                png = ios::screenshot(&device.id).await?;
            }

            png
        }
        _ => {
            let serial = attached_serial(device)
                .await
                .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

            let display = adb::active_display(&serial).await;

            // exec-out keeps the pty's LF -> CRLF translation away from the PNG,
            // but folds the device's stderr into the same stream, so the
            // redirect has to run on the device, inside the quoted command.
            let remote = match display {
                Some(id) => format!("screencap -p -d {id} 2>/dev/null"),
                None => "screencap -p 2>/dev/null".to_string(),
            };

            let (_, bytes) = adb::run_bytes(&["-s", &serial, "exec-out", &remote]).await?;

            bytes
        }
    };

    if !is_png(&png) {
        bail!("capture failed on {} (is it awake and unlocked?)", device.label);
    }

    match sink {
        Sink::Stdout => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&png)?;
            stdout.flush()?;

            Ok("wrote PNG to stdout".into())
        }
        Sink::File(path) => {
            std::fs::write(path, &png).with_context(|| format!("writing {path}"))?;

            Ok(format!("saved {path}"))
        }
        Sink::Clipboard => {
            let mut cmd = tokio::process::Command::new("wl-copy");

            cmd.args(["--type", "image/png"]);

            // wl-copy forks a daemon that keeps serving the selection, so the
            // image only lives as long as that process does. Leaving it in this
            // process group means closing the terminal SIGHUPs the clipboard
            // contents away along with the TUI.
            cmd.process_group(0);

            pipe_to(&mut cmd, &png, "wl-copy").await?;

            Ok(format!("copied {} to the clipboard", device.label))
        }
    }
}

/// Feeds `bytes` to `cmd` on stdin and waits for it to exit. Only the front
/// process is waited on: anything it forks off inherits the stderr pipe and can
/// outlive it, so that pipe is read on failure alone.
async fn pipe_to(cmd: &mut tokio::process::Command, bytes: &[u8], what: &str) -> Result<()> {
    cmd.stdin(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("running {what}"))?;

    let wrote = match child.stdin.take() {
        Some(mut stdin) => {
            use tokio::io::AsyncWriteExt as _;
            stdin.write_all(bytes).await.and(stdin.shutdown().await)
        }
        None => Ok(()),
    };

    let mut stderr = child.stderr.take();
    let status = child.wait().await?;

    if !status.success() {
        let mut msg = String::new();

        if let Some(stderr) = stderr.as_mut() {
            use tokio::io::AsyncReadExt as _;
            let _ = stderr.read_to_string(&mut msg).await;
        }

        bail!("{what} failed: {}", msg.trim());
    }

    wrote.with_context(|| format!("writing to {what}"))?;

    Ok(())
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.len() > 8 && bytes[..4] == PNG_MAGIC
}

/// Logcat and oslog both take over the terminal, so they are handed back as a
/// command for the caller to exec once it has torn down whatever UI it owns.
pub async fn logs_command(device: &Device, app: &str) -> Result<std::process::Command> {
    if !app
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        || app.is_empty()
    {
        bail!("invalid app id: {app}");
    }

    if device.platform == Platform::Ios {
        return ios::logs_command(&device.id, app);
    }

    let serial = attached_serial(device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    let pid = adb::pidof(&serial, app)
        .await
        .ok_or_else(|| anyhow!("app is not running: {app}"))?;

    let mut cmd = std::process::Command::new("adb");
    cmd.args(["-s", &serial, "logcat", &format!("--pid={pid}")]);

    Ok(cmd)
}

/// scrcpy owns a window, not the terminal, so it is detached and the caller
/// keeps running.
pub async fn mirror(device: &Device) -> Result<String> {
    if device.platform == Platform::Ios {
        bail!("scrcpy cannot mirror an iPhone");
    }

    let serial = attached_serial(device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    tokio::process::Command::new("scrcpy")
        .args(["-s", &serial])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("running scrcpy")?;

    Ok(format!("mirroring {}", device.label))
}

pub async fn install(device: &Device, apk: &Path, rep: &Reporter) -> Result<String> {
    if device.platform == Platform::Ios {
        bail!("cannot install an apk on an iPhone");
    }

    if !apk.exists() {
        bail!("no such file: {}", apk.display());
    }

    let serial = attached_serial(device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    rep.try_(format!("install {}", apk.display()));

    let path = apk.to_string_lossy().to_string();
    let out = adb::run(&["-s", &serial, "install", "-r", &path]).await?;

    if !out.ok() || out.stdout.contains("Failure") {
        bail!(
            "{}",
            out.stderr
                .trim()
                .lines()
                .chain(out.stdout.trim().lines())
                .find(|l| !l.is_empty())
                .unwrap_or("install failed")
        );
    }

    Ok(format!("installed on {}", device.label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_before_a_forked_grandchild_exits() {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", "cat >/dev/null; sleep 20 & exit 0"]);

        let start = std::time::Instant::now();

        pipe_to(&mut cmd, b"payload", "sh").await.unwrap();

        assert!(
            start.elapsed() < Duration::from_secs(5),
            "waited on a process that inherited stderr and outlived the child"
        );
    }

    #[tokio::test]
    async fn reports_the_stderr_of_a_failing_child() {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", "cat >/dev/null; echo boom >&2; exit 1"]);

        let err = pipe_to(&mut cmd, b"payload", "wl-copy").await.unwrap_err();

        assert_eq!(err.to_string(), "wl-copy failed: boom");
    }
}
