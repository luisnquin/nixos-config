use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};

use crate::model::{Device, Platform};
use crate::ssh;

/// tunneld's own port on the host that runs it. An iPhone is never reachable
/// from here directly: it talks to a mac, and that mac holds the RemoteXPC
/// tunnel iOS 17+ requires.
pub const TUNNELD_PORT: u16 = 49151;

pub async fn devices(host: &str) -> Vec<Device> {
    let remote = format!("curl -s --max-time 3 http://127.0.0.1:{TUNNELD_PORT}/");
    let text = ssh::text(host, &remote, Duration::from_secs(12)).await;

    let Ok(tunnels) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&text) else {
        return Vec::new();
    };

    tunnels
        .into_keys()
        .map(|udid| {
            let mut d = Device::new(
                format!("{host}/{udid}"),
                format!("iPhone ({host})"),
                Platform::Ios,
            );

            d.model = "iPhone".into();
            d.host = Some(host.to_string());
            d.add_alias(udid);

            d
        })
        .collect()
}

/// The udid half of the namespaced device id.
pub fn udid(device: &Device) -> Result<&str> {
    let udid = device
        .id
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("no udid in {}", device.id))?;

    check_udid(udid)?;

    Ok(udid)
}

/// pymobiledevice3's bare `screenshot` targets "the first USB device" and fails
/// whenever the phone is only on wifi, so the tunnel has to be named explicitly.
pub async fn screenshot(host: &str, udid: &str) -> Result<Vec<u8>> {
    check_udid(udid)?;

    const SCRIPT: &str = r#"d="$(mktemp -d)"
pymobiledevice3 developer dvt screenshot --tunnel "$1" "$d/shot.png" >/dev/null 2>&1
cat "$d/shot.png" 2>/dev/null
rm -rf "$d""#;

    // pymobiledevice3 blocks forever on a locked device or a half-dead tunnel,
    // and in the TUI that wedges the busy guard for the rest of the session.
    ssh::output(ssh::script(host, SCRIPT, &[udid]), Duration::from_secs(45))
        .await
        .map_err(|_| anyhow!("{host} did not answer in time; is the iPhone unlocked?"))
}

pub fn logs_command(host: &str, udid: &str, app: &str) -> Result<std::process::Command> {
    check_udid(udid)?;

    const SCRIPT: &str = r#"pid="$(pymobiledevice3 developer dvt process-id-for-bundle-id --tunnel "$1" "$2" 2>/dev/null)"
case "$pid" in
  ""|*[!0-9]*) echo "phone: app is not running: $2" >&2; exit 1;;
esac
exec pymobiledevice3 developer dvt oslog --tunnel "$1" "$pid""#;

    let mut cmd = std::process::Command::new("ssh");

    cmd.args(["-t", host])
        .arg(ssh::remote(SCRIPT, &[udid, app]));

    Ok(cmd)
}

fn check_udid(udid: &str) -> Result<()> {
    if udid.is_empty() || !udid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("invalid udid: {udid}");
    }

    Ok(())
}
