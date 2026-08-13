use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

use crate::model::{Device, Platform};
use crate::ssh;

/// A simulator is the one device class that cannot be federated. adb has a
/// server to point a client at and tunneld speaks HTTP, but CoreSimulator is
/// launchd and mach ports: macOS-local, with no socket to forward. Every
/// operation here is therefore a command run on the host that owns it.
#[derive(Deserialize)]
struct ListJson {
    #[serde(default)]
    devices: HashMap<String, Vec<SimJson>>,
}

#[derive(Deserialize)]
struct SimJson {
    #[serde(default)]
    udid: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    state: String,
}

pub async fn devices(host: &str) -> Vec<Device> {
    let bytes = ssh::output(
        {
            let mut cmd = ssh::command(host);
            cmd.arg("xcrun simctl list devices booted -j");
            cmd
        },
        Duration::from_secs(20),
    )
    .await;

    let Ok(bytes) = bytes else {
        return Vec::new();
    };

    let Ok(list) = serde_json::from_slice::<ListJson>(&bytes) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for (runtime, sims) in list.devices {
        for sim in sims {
            if sim.state != "Booted" || sim.udid.is_empty() {
                continue;
            }

            let mut device = Device::new(
                format!("{host}/{}", sim.udid),
                sim.name.clone(),
                Platform::Simulator,
            );

            device.model = runtime_label(&runtime);
            device.host = Some(host.to_string());
            device.add_alias(sim.udid);

            out.push(device);
        }
    }

    out.sort_by(|a, b| a.label.cmp(&b.label));

    out
}

/// `com.apple.CoreSimulator.SimRuntime.iOS-26-5` is the runtime key; the last
/// segment is the only part worth a column.
fn runtime_label(runtime: &str) -> String {
    let Some(tail) = runtime.rsplit('.').next() else {
        return String::new();
    };

    match tail.split_once('-') {
        Some((os, version)) => format!("{os} {}", version.replace('-', ".")),
        None => tail.to_string(),
    }
}

/// The udid half of the namespaced device id.
pub fn udid(device: &Device) -> Result<&str> {
    let udid = device
        .id
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("no udid in {}", device.id))?;

    check(udid)?;

    Ok(udid)
}

fn check(udid: &str) -> Result<()> {
    if udid.is_empty() || !udid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("invalid udid: {udid}");
    }

    Ok(())
}

/// simctl writes a screenshot to a path, never to a stream, so the file has to
/// be made and read back on the host rather than piped straight out.
pub async fn screenshot(host: &str, udid: &str) -> Result<Vec<u8>> {
    check(udid)?;

    const SCRIPT: &str = r#"f="$(mktemp -t phone-shot).png"
xcrun simctl io "$1" screenshot --type=png "$f" >/dev/null 2>&1
cat "$f" 2>/dev/null
rm -f "$f""#;

    ssh::output(ssh::script(host, SCRIPT, &[udid]), Duration::from_secs(45))
        .await
        .map_err(|_| anyhow!("{host} did not answer in time"))
}

pub fn logs_command(host: &str, udid: &str, app: &str) -> Result<std::process::Command> {
    check(udid)?;

    const SCRIPT: &str = r#"exec xcrun simctl spawn "$1" log stream --style compact \
  --predicate 'subsystem == "'"$2"'" OR process == "'"$2"'"'"#;

    let mut cmd = std::process::Command::new("ssh");

    cmd.args(["-t", host])
        .arg(ssh::remote(SCRIPT, &[udid, app]));

    Ok(cmd)
}

/// Installs a `.app` bundle. Unlike adb — where the client reads the file and
/// pushes the bytes through the server — simctl only ever opens a path on its
/// own filesystem, so the bundle is copied over first.
pub async fn install(host: &str, udid: &str, path: &std::path::Path) -> Result<()> {
    check(udid)?;

    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("{} has no file name", path.display()))?
        .to_string_lossy()
        .to_string();

    let remote = format!("/tmp/phone-install/{name}");

    let mkdir = ssh::command(host)
        .arg("mkdir -p /tmp/phone-install")
        .status();

    tokio::time::timeout(Duration::from_secs(20), mkdir).await??;

    let scp = tokio::process::Command::new("scp")
        .args(["-q", "-r", "-o", "BatchMode=yes"])
        .arg(path)
        .arg(format!("{host}:{remote}"))
        .status();

    let status = tokio::time::timeout(Duration::from_secs(180), scp).await??;

    if !status.success() {
        bail!("could not copy {} to {host}", path.display());
    }

    let out = ssh::script(
        host,
        r#"xcrun simctl install "$1" "$2" 2>&1"#,
        &[udid, &remote],
    )
    .output();

    let out = tokio::time::timeout(Duration::from_secs(120), out).await??;

    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stdout).trim());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_runtime_out_of_the_key() {
        assert_eq!(
            runtime_label("com.apple.CoreSimulator.SimRuntime.iOS-26-5"),
            "iOS 26.5"
        );
    }

    #[test]
    fn rejects_a_udid_that_could_carry_shell_syntax() {
        assert!(check("3F83A110-39DD-445A-AD47-7D487A0C818B").is_ok());
        assert!(check("$(rm -rf /)").is_err());
    }
}
