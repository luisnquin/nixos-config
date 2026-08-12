use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::process::Command;

use crate::model::{Device, Platform};

/// The iPhone is never reachable from this host directly: it talks to the mac,
/// and the mac keeps the RemoteXPC tunnel that iOS 17+ requires.
pub const HOST: &str = "rose";
const TUNNELD_URL: &str = "http://127.0.0.1:49151/";

fn ssh(args: &[&str]) -> Command {
    let mut cmd = Command::new("ssh");

    cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=3"]);
    cmd.args(args);
    cmd.stdin(Stdio::null());

    cmd
}

pub async fn devices() -> Vec<Device> {
    let out = tokio::time::timeout(
        Duration::from_secs(8),
        ssh(&[HOST, &format!("curl -s --max-time 3 {TUNNELD_URL}")])
            .stderr(Stdio::null())
            .output(),
    )
    .await;

    let Ok(Ok(out)) = out else {
        return Vec::new();
    };

    let Ok(tunnels) = serde_json::from_slice::<HashMap<String, serde_json::Value>>(&out.stdout)
    else {
        return Vec::new();
    };

    tunnels
        .into_keys()
        .map(|udid| {
            let mut d = Device::new(udid.clone(), format!("iPhone ({HOST})"), Platform::Ios);
            d.model = "iPhone".into();
            d
        })
        .collect()
}

/// pymobiledevice3's bare `screenshot` targets "the first USB device" and fails
/// whenever the phone is only on wifi, so the tunnel has to be named explicitly.
pub async fn screenshot(udid: &str) -> Result<Vec<u8>> {
    check_udid(udid)?;

    let remote = r#"d="$(mktemp -d)"; pymobiledevice3 developer dvt screenshot --tunnel "$1" "$d/shot.png" >/dev/null 2>&1; cat "$d/shot.png" 2>/dev/null; rm -rf "$d""#;

    // pymobiledevice3 blocks forever on a locked device or a half-dead tunnel,
    // and in the TUI that wedges the busy guard for the rest of the session.
    let out = tokio::time::timeout(
        Duration::from_secs(45),
        ssh(&[HOST, &format!("sh -c '{remote}' sh '{udid}'")])
            .stderr(Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| anyhow!("{HOST} did not answer in time; is the iPhone unlocked?"))??;

    Ok(out.stdout)
}

pub fn logs_command(udid: &str, app: &str) -> Result<std::process::Command> {
    check_udid(udid)?;

    let remote = r#"pid="$(pymobiledevice3 developer dvt process-id-for-bundle-id --tunnel "$1" "$2" 2>/dev/null)"; case "$pid" in ""|*[!0-9]*) echo "phone: app is not running: $2" >&2; exit 1;; esac; exec pymobiledevice3 developer dvt oslog --tunnel "$1" "$pid""#;

    let mut cmd = std::process::Command::new("ssh");

    cmd.args(["-t", HOST, &format!("sh -c '{remote}' sh '{udid}' '{app}'")]);

    Ok(cmd)
}

fn check_udid(udid: &str) -> Result<()> {
    if udid.is_empty() || !udid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("invalid udid: {udid}");
    }

    Ok(())
}
