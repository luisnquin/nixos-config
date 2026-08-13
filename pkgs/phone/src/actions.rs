use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};

use crate::adb::{self, Server};
use crate::connect::{attached_serial, Reporter};
use crate::model::{Device, Platform};
use crate::{ios, simctl};

const PNG_MAGIC: [u8; 4] = [0x89, b'P', b'N', b'G'];

/// The machine a hosted device hangs off. Nothing here can reach one without
/// it, so a device that lost its host is an error rather than a fallback to
/// this machine, which would silently drive the wrong device.
fn host_of(device: &Device) -> Result<&str> {
    device
        .host
        .as_deref()
        .filter(|h| !h.is_empty())
        .ok_or_else(|| anyhow!("{} is not on any known host", device.label))
}

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
pub async fn screenshot(
    server: &Server,
    device: &Device,
    sink: &Sink,
    rep: &Reporter,
) -> Result<String> {
    let png = match device.platform {
        Platform::Ios => {
            let host = host_of(device)?;
            let udid = ios::udid(device)?;

            let mut png = ios::screenshot(host, udid).await?;

            if !is_png(&png) {
                // tunneld may still be coming up right after a mac reboot
                rep.note("no image yet, retrying the tunnel");
                tokio::time::sleep(Duration::from_secs(3)).await;
                png = ios::screenshot(host, udid).await?;
            }

            png
        }
        Platform::Simulator => simctl::screenshot(host_of(device)?, simctl::udid(device)?).await?,
        _ => {
            let serial = attached_serial(server, device)
                .await
                .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

            let display = adb::active_display(server, &serial).await;

            // exec-out keeps the pty's LF -> CRLF translation away from the PNG,
            // but folds the device's stderr into the same stream, so the
            // redirect has to run on the device, inside the quoted command.
            let remote = match display {
                Some(id) => format!("screencap -p -d {id} 2>/dev/null"),
                None => "screencap -p 2>/dev/null".to_string(),
            };

            let (_, bytes) = adb::run_bytes(server, &["-s", &serial, "exec-out", &remote]).await?;

            bytes
        }
    };

    if !is_png(&png) {
        bail!(
            "capture failed on {} (is it awake and unlocked?)",
            device.label
        );
    }

    let (msg, body, icon) = match sink {
        Sink::Stdout => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&png)?;
            stdout.flush()?;

            // the caller is piping the bytes somewhere, so a popup is noise
            return Ok("wrote PNG to stdout".into());
        }
        Sink::File(path) => {
            std::fs::write(path, &png).with_context(|| format!("writing {path}"))?;

            // notify-send resolves an icon as a file only from an absolute path
            let icon = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));

            let msg = format!("saved {path}");

            (msg.clone(), msg, Some(icon))
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

            (
                format!("copied {} to the clipboard", device.label),
                "copied to the clipboard".to_string(),
                preview(&png),
            )
        }
    };

    notify(&device.label, &body, icon.as_deref()).await;

    Ok(msg)
}

/// A notification daemon reads the icon off disk, so a capture that only went to
/// the clipboard still needs a file to point at. One fixed name under the runtime
/// dir: tmpfs, dies with the session, and every capture overwrites the last one.
fn preview(png: &[u8]) -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    preview_in(&dir.join("phone"), png)
}

fn preview_in(dir: &Path, png: &[u8]) -> Option<PathBuf> {
    std::fs::create_dir_all(dir).ok()?;

    let path = dir.join("preview.png");
    std::fs::write(&path, png).ok()?;

    Some(path)
}

/// Best effort: a missing notify-send or a dead bus must not fail a capture that
/// already landed.
async fn notify(summary: &str, body: &str, icon: Option<&Path>) {
    let mut cmd = tokio::process::Command::new("notify-send");

    cmd.args(["--app-name=phone", "--expire-time=4000"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(icon) = icon {
        cmd.arg("--icon").arg(icon);
    }

    cmd.args(["--", summary, body]);

    let _ = cmd.status().await;
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
pub async fn logs_command(
    server: &Server,
    device: &Device,
    app: &str,
) -> Result<std::process::Command> {
    if !app
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        || app.is_empty()
    {
        bail!("invalid app id: {app}");
    }

    match device.platform {
        Platform::Ios => return ios::logs_command(host_of(device)?, ios::udid(device)?, app),
        Platform::Simulator => {
            return simctl::logs_command(host_of(device)?, simctl::udid(device)?, app)
        }
        _ => {}
    }

    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    let pid = adb::pidof(server, &serial, app)
        .await
        .ok_or_else(|| anyhow!("app is not running: {app}"))?;

    let mut cmd = server.command();
    cmd.args(["-s", &serial, "logcat", &format!("--pid={pid}")]);

    Ok(cmd)
}

/// scrcpy owns a window, not the terminal, so it is detached and the caller
/// keeps running.
///
/// It works against a remote server unchanged: the encoder runs on the device
/// and the H.264 stream rides the same adb connection as everything else, so
/// nothing but the frames crosses the network.
pub async fn mirror(server: &Server, device: &Device) -> Result<String> {
    if device.platform.is_hosted() {
        bail!("scrcpy cannot mirror {}", device.platform);
    }

    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    let mut cmd = tokio::process::Command::new("scrcpy");

    // scrcpy runs its own adb, which takes no flag of ours
    if let Some((key, value)) = server.env() {
        cmd.env(key, value);
    }

    cmd.args(["-s", &serial])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("running scrcpy")?;

    Ok(format!("mirroring {}", device.label))
}

pub async fn install(
    server: &Server,
    device: &Device,
    bundle: &Path,
    rep: &Reporter,
) -> Result<String> {
    if device.platform == Platform::Ios {
        bail!("cannot install on an iPhone from here");
    }

    if !bundle.exists() {
        bail!("no such file: {}", bundle.display());
    }

    if device.platform == Platform::Simulator {
        rep.try_(format!("install {}", bundle.display()));
        simctl::install(host_of(device)?, simctl::udid(device)?, bundle).await?;

        return Ok(format!("installed on {}", device.label));
    }

    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    rep.try_(format!("install {}", bundle.display()));

    let path = bundle.to_string_lossy().to_string();
    let out = adb::run(server, &["-s", &serial, "install", "-r", &path]).await?;

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

    #[test]
    fn every_preview_reuses_the_same_file() {
        let dir = std::env::temp_dir().join(format!("phone-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let first = preview_in(&dir, b"first").unwrap();
        let second = preview_in(&dir, b"second").unwrap();

        assert_eq!(first, second);
        assert_eq!(std::fs::read(&second).unwrap(), b"second");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_preview_path_is_absolute_for_notify_send() {
        let dir = std::env::temp_dir().join(format!("phone-preview-abs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let path = preview_in(&dir, b"png").unwrap();

        assert!(
            path.is_absolute(),
            "notify-send would treat {path:?} as an icon name"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn reports_the_stderr_of_a_failing_child() {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", "cat >/dev/null; echo boom >&2; exit 1"]);

        let err = pipe_to(&mut cmd, b"payload", "wl-copy").await.unwrap_err();

        assert_eq!(err.to_string(), "wl-copy failed: boom");
    }
}
