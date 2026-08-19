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

/// Every simulator that exists on `host`, each with whether it is running.
/// Asking only for the booted ones would hide what could be started, which is
/// the whole of what a caller needs before it can boot anything.
pub async fn devices(host: &str) -> Vec<(Device, bool)> {
    let bytes = ssh::output(
        {
            let mut cmd = ssh::command(host);
            cmd.arg("xcrun simctl list devices available -j");
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
            if sim.udid.is_empty() {
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

            out.push((device, sim.state == "Booted"));
        }
    }

    out.sort_by(|a, b| a.0.label.cmp(&b.0.label));

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

    // the remote status is read off stderr for the same reason `run` does it: a
    // brokered ssh session reports success whatever the command did
    let script =
        format!("xcrun simctl install \"$1\" \"$2\" >&2\nprintf '{STATUS}%s\\n' \"$?\" >&2");

    let out = ssh::script(host, &script, &[udid, &remote]).output();
    let out = tokio::time::timeout(Duration::from_secs(120), out).await??;

    match outcome(&out.stderr) {
        (Some(0), _) => Ok(()),
        (_, reason) if !reason.is_empty() => bail!("{reason}"),
        _ => bail!("{host} did not say whether the install landed"),
    }
}

/// Reading and pressing need the CoreSimulator and SimulatorKit frameworks,
/// which `xcrun simctl` does not expose and which only load on macOS. The host
/// carries its own `phone` linked against them, so what runs there is the same
/// verb by the same name — this side only ferries it over ssh.
const TOOL: &str = "phone";

/// Android key names, because the caller says `phone key home` whatever is
/// answering. Only the ones a handset and a simulator both have: `back` is
/// deliberately absent, since iOS has no such button and a key silently swapped
/// for a near miss is worse than one refused.
const KEYS: &[(&str, &str)] = &[
    ("app_switch", "app-switcher"),
    ("del", "delete"),
    ("dpad_down", "down"),
    ("dpad_left", "left"),
    ("dpad_right", "right"),
    ("dpad_up", "up"),
    ("enter", "enter"),
    ("escape", "escape"),
    ("forward_del", "delete"),
    ("home", "home"),
    ("power", "lock"),
    ("sleep", "lock"),
    ("space", "space"),
    ("tab", "tab"),
    ("volume_down", "volume-down"),
    ("volume_up", "volume-up"),
];

fn button(name: &str) -> Result<&'static str> {
    let name = name.trim().to_lowercase();
    let name = name.strip_prefix("keycode_").unwrap_or(&name);

    match KEYS.iter().find(|(android, _)| *android == name) {
        Some((_, ios)) => Ok(ios),
        None => bail!(
            "a simulator has no '{name}' key (it takes: {})",
            KEYS.iter()
                .map(|(android, _)| *android)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Marks the line the remote status is written on.
const STATUS: &str = "phone-status:";

/// Splits a remote run's stderr into the status the command exited with and the
/// reason it printed. `None` for a session that never reached the command.
fn outcome(stderr: &[u8]) -> (Option<i32>, String) {
    let stderr = String::from_utf8_lossy(stderr);
    let mut code = None;
    let mut reason = Vec::new();

    for line in stderr.lines() {
        match line.strip_prefix(STATUS) {
            Some(value) => code = value.trim().parse().ok(),
            None => reason.push(line.strip_prefix("phone: ").unwrap_or(line)),
        }
    }

    (code, reason.join("\n").trim().to_string())
}

/// One `phone` run on the host. The remote refuses an unknown key or an
/// untypeable character, and that reason is the whole answer, so it is carried
/// back rather than flattened into a failure.
///
/// The exit code is written into stderr and read back off it because not every
/// ssh session carries one: a brokered one reports success whatever the command
/// did, which would turn every refusal into an empty success here.
async fn run(host: &str, args: &[&str], limit: Duration) -> Result<Vec<u8>> {
    let script = format!("{TOOL} \"$@\"\nprintf '{STATUS}%s\\n' \"$?\" >&2");
    let out = ssh::script(host, &script, args).output();
    let out = tokio::time::timeout(limit, out)
        .await
        .map_err(|_| anyhow!("{host} did not answer in time"))??;

    let (code, reason) = outcome(&out.stderr);

    match code {
        Some(0) => Ok(out.stdout),
        // what a shell reports for a command it could not find
        Some(127) => bail!("{host} has no {TOOL} to read or press a simulator with"),
        Some(_) if !reason.is_empty() => bail!("{reason}"),
        Some(code) => bail!("{TOOL} on {host} exited {code}"),
        None if reason.is_empty() => bail!("{host} did not run {TOOL}"),
        None => bail!("{reason}"),
    }
}

pub async fn snapshot(host: &str, udid: &str) -> Result<Vec<crate::a11y::Node>> {
    check(udid)?;

    let bytes = run(host, &["snapshot", udid], Duration::from_secs(45)).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

/// The panel in points and the factor that maps them to the pixels a screenshot
/// comes back in. Taps and element bounds are both in points, so nothing else
/// needs it — until a crop taken from bounds has to be found in an image.
pub async fn size(host: &str, udid: &str) -> Result<crate::a11y::Size> {
    check(udid)?;

    let bytes = run(host, &["size", udid], Duration::from_secs(30)).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn swipe(
    host: &str,
    udid: &str,
    from: (i32, i32),
    to: (i32, i32),
    ms: u64,
) -> Result<()> {
    check(udid)?;

    let args = [from.0, from.1, to.0, to.1].map(|v| v.to_string());
    let ms = ms.to_string();

    run(
        host,
        &["swipe", udid, &args[0], &args[1], &args[2], &args[3], &ms],
        Duration::from_secs(30 + ms.parse::<u64>().unwrap_or(0) / 1000),
    )
    .await?;

    Ok(())
}

pub async fn tap(host: &str, udid: &str, x: i32, y: i32) -> Result<()> {
    check(udid)?;

    let (x, y) = (x.to_string(), y.to_string());

    run(host, &["tap", udid, &x, &y], Duration::from_secs(30)).await?;

    Ok(())
}

pub async fn type_text(host: &str, udid: &str, text: &str) -> Result<()> {
    check(udid)?;

    // One key event per character on the far side, so the budget grows with the
    // string rather than being a flat guess.
    let limit = Duration::from_secs(30 + text.len() as u64 / 4);

    run(host, &["text", udid, text], limit).await?;

    Ok(())
}

pub async fn key(host: &str, udid: &str, name: &str) -> Result<()> {
    check(udid)?;

    run(host, &["key", udid, button(name)?], Duration::from_secs(30)).await?;

    Ok(())
}

/// Boots `udid` and returns once the system is up, not once the process is.
/// `bootstatus` is what waits; `open` is only there so the window appears, and
/// a headless boot is still a usable device without it.
pub async fn boot(host: &str, udid: &str) -> Result<()> {
    const SCRIPT: &str = r#"xcrun simctl boot "$1" 2>/dev/null
open -a Simulator 2>/dev/null
xcrun simctl bootstatus "$1" -b >&2"#;

    check(udid)?;

    let out = ssh::script(host, SCRIPT, &[udid])
        .output()
        .await
        .map_err(|e| anyhow!("{host}: {e}"))?;

    if !out.status.success() {
        bail!("{host}: {}", String::from_utf8_lossy(&out.stderr).trim());
    }

    Ok(())
}

pub async fn shutdown(host: &str, udid: &str) -> Result<()> {
    check(udid)?;

    let out = ssh::script(host, r#"exec xcrun simctl shutdown "$1""#, &[udid])
        .output()
        .await
        .map_err(|e| anyhow!("{host}: {e}"))?;

    if !out.status.success() {
        bail!("{host}: {}", String::from_utf8_lossy(&out.stderr).trim());
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

    #[test]
    fn takes_the_key_by_its_android_name() {
        assert_eq!(button("home").unwrap(), "home");
        assert_eq!(button("KEYCODE_VOLUME_UP").unwrap(), "volume-up");
        assert_eq!(button("app_switch").unwrap(), "app-switcher");
    }

    /// iOS navigates back by a gesture or an on-screen control, so there is no
    /// button to press and nothing to quietly substitute.
    #[test]
    fn refuses_a_key_the_platform_does_not_have() {
        let err = button("back").unwrap_err().to_string();

        assert!(err.contains("no 'back' key"), "{err}");
        assert!(err.contains("home"), "the alternatives are listed: {err}");
    }

    #[test]
    fn reads_the_remote_status_off_the_stream_that_carries_it() {
        assert_eq!(outcome(b"phone-status:0\n"), (Some(0), String::new()));
        assert_eq!(
            outcome(b"phone: unknown key: wiggle\nphone-status:1\n"),
            (Some(1), "unknown key: wiggle".to_string())
        );
    }

    /// A session that never reached the command leaves no status behind, and
    /// reporting that as a clean run would call every failure a success.
    #[test]
    fn tells_a_missing_status_apart_from_a_zero_one() {
        assert_eq!(outcome(b""), (None, String::new()));
        assert_eq!(
            outcome(b"ssh: connect: host is down\n"),
            (None, "ssh: connect: host is down".to_string())
        );
    }
}
