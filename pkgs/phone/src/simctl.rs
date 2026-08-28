use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

use crate::model::{Device, Platform};
use crate::ssh::{self, Where};

/// A simulator is the one device class that cannot be federated. adb has a
/// server to point a client at and tunneld speaks HTTP, but CoreSimulator is
/// launchd and mach ports: macOS-local, with no socket to forward. Every
/// operation here is therefore a command run on the host that owns it, which is
/// this machine when the simulator is on it and reached over ssh when it is not.
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

/// Every simulator that exists on `at`, each with whether it is running.
/// Asking only for the booted ones would hide what could be started, which is
/// the whole of what a caller needs before it can boot anything.
pub async fn devices(at: &Where) -> Vec<(Device, bool)> {
    let bytes = ssh::output(
        at.run("xcrun simctl list devices available -j", &[]),
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

            // namespaced by host because two machines can hold simulators with
            // the same udid only by accident, but the same machine seen under two
            // names would otherwise be two devices. Nothing prefixes the local
            // ones: there is no second machine here to tell them apart from.
            let mut device = Device::new(
                match at.host() {
                    Some(host) => format!("{host}/{}", sim.udid),
                    None => sim.udid.clone(),
                },
                sim.name.clone(),
                Platform::Simulator,
            );

            device.model = runtime_label(&runtime);
            device.host = at.host().map(str::to_string);
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
///
/// Everything simctl says is kept. Silencing it left nothing to report but the
/// ssh session, which for a device that is merely off means waiting out the
/// timeout and then blaming the host — and on a failure that did not hang, an
/// empty `cat` that read as a screenshot.
pub async fn screenshot(at: &Where, udid: &str) -> Result<Vec<u8>> {
    check(udid)?;

    // `mktemp -t` makes the file without the suffix, so both names go. `|| exit`
    // so the status that comes back is the one simctl set rather than the one
    // `cat` sets for a file simctl never wrote.
    const SCRIPT: &str = r#"f="$(mktemp -t phone-shot)"
trap 'rm -f "$f" "$f.png"' EXIT
xcrun simctl io "$1" screenshot --type=png "$f.png" >&2 || exit
cat "$f.png""#;

    // the capture itself is instant; what this waits on is ~3 MB of PNG coming
    // back over ssh, and that link is not always a fast one. Measured at 54 kB/s
    // to rose, a panel takes about a minute, so 45s was a coin toss.
    let mut ran = at.exec(SCRIPT, &[udid], Duration::from_secs(180)).await?;
    let png = std::mem::take(&mut ran.stdout);

    landed(at, "simctl io screenshot", ran)?;

    if png.is_empty() {
        bail!("{} wrote no image of the simulator", at.label());
    }

    Ok(png)
}

pub fn logs_command(at: &Where, udid: &str, app: &str) -> Result<std::process::Command> {
    check(udid)?;

    const SCRIPT: &str = r#"exec xcrun simctl spawn "$1" log stream --style compact \
  --predicate 'subsystem == "'"$2"'" OR process == "'"$2"'"'"#;

    // `-t` so the stream ends when this terminal does; a local one already
    // shares the terminal it was started from and needs no pty asked for.
    Ok(match at.host() {
        Some(host) => {
            let mut cmd = std::process::Command::new("ssh");

            cmd.args(["-t", host]).arg(ssh::remote(SCRIPT, &[udid, app]));

            cmd
        }
        None => {
            let mut cmd = std::process::Command::new("sh");

            cmd.arg("-c").arg(SCRIPT).arg("sh").arg(udid).arg(app);

            cmd
        }
    })
}

/// Installs a `.app` bundle. Unlike adb — where the client reads the file and
/// pushes the bytes through the server — simctl only ever opens a path on its
/// own filesystem, so the bundle is copied over first.
pub async fn install(at: &Where, udid: &str, path: &std::path::Path) -> Result<()> {
    check(udid)?;

    let bundle = match at.host() {
        // already on the filesystem simctl opens
        None => path.display().to_string(),
        Some(host) => {
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

            remote
        }
    };

    // simctl writes what went wrong to stdout as often as to stderr, and only
    // one of the two carries the status back
    let ran = at
        .exec(
            "xcrun simctl install \"$1\" \"$2\" >&2",
            &[udid, &bundle],
            Duration::from_secs(120),
        )
        .await?;

    match (ran.status, ran.said) {
        (ssh::Status::Code(0), _) => Ok(()),
        (_, reason) if !reason.is_empty() => bail!("{reason}"),
        _ => bail!("{} did not say whether the install landed", at.label()),
    }
}

/// Reading and pressing need the CoreSimulator and SimulatorKit frameworks,
/// which `xcrun simctl` does not expose and which only load on macOS. The host
/// carries its own binary linked against them, so what runs there is the same
/// verb by the same name — this side only ferries it.
const TOOL: &str = "phone-receiver";

/// The bridge answered to `phone` before this CLI could run on macOS itself,
/// where that name is now the CLI. Both are installed together, so the fallback
/// only matters for the window where one side has been updated and the other
/// has not — and on that host `phone` is still the bridge.
const CALL: &str = r#"tool="$(command -v phone-receiver 2>/dev/null)"
exec "${tool:-phone}" "$@""#;

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

/// One `phone` run on the host. The remote refuses an unknown key or an
/// untypeable character, and that reason is the whole answer, so it is carried
/// back rather than flattened into a failure.
async fn run(at: &Where, args: &[&str], limit: Duration) -> Result<Vec<u8>> {
    let ran = at.exec(CALL, args, limit).await?;
    let host = at.label();

    match ran.status {
        ssh::Status::Code(0) => Ok(ran.stdout),
        // what a shell reports for a command it could not find
        ssh::Status::Code(127) => bail!("{host} has no {TOOL} to read or press a simulator with"),
        ssh::Status::Code(_) if !ran.said.is_empty() => bail!("{}", ran.said),
        ssh::Status::Code(code) => bail!("{TOOL} on {host} exited {code}"),
        ssh::Status::Garbled(said) => {
            bail!("{host} ended {TOOL} with '{said}' rather than a status")
        }
        ssh::Status::Missing if ran.said.is_empty() => bail!("{host} did not run {TOOL}"),
        ssh::Status::Missing => bail!("{}", ran.said),
    }
}

pub async fn snapshot(at: &Where, udid: &str) -> Result<Vec<crate::a11y::Node>> {
    check(udid)?;

    let bytes = run(at, &["snapshot", udid], Duration::from_secs(45)).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

/// The panel in points and the factor that maps them to the pixels a screenshot
/// comes back in. Taps and element bounds are both in points, so nothing else
/// needs it — until a crop taken from bounds has to be found in an image.
pub async fn size(at: &Where, udid: &str) -> Result<crate::a11y::Size> {
    check(udid)?;

    let bytes = run(at, &["size", udid], Duration::from_secs(30)).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn swipe(
    at: &Where,
    udid: &str,
    from: (i32, i32),
    to: (i32, i32),
    ms: u64,
) -> Result<()> {
    check(udid)?;

    let args = [from.0, from.1, to.0, to.1].map(|v| v.to_string());
    let ms = ms.to_string();

    run(
        at,
        &["swipe", udid, &args[0], &args[1], &args[2], &args[3], &ms],
        Duration::from_secs(30 + ms.parse::<u64>().unwrap_or(0) / 1000),
    )
    .await?;

    Ok(())
}

pub async fn tap(at: &Where, udid: &str, x: i32, y: i32) -> Result<()> {
    check(udid)?;

    let (x, y) = (x.to_string(), y.to_string());

    run(at, &["tap", udid, &x, &y], Duration::from_secs(30)).await?;

    Ok(())
}

pub async fn type_text(at: &Where, udid: &str, text: &str) -> Result<()> {
    check(udid)?;

    // One key event per character on the far side, so the budget grows with the
    // string rather than being a flat guess.
    let limit = Duration::from_secs(30 + text.len() as u64 / 4);

    run(at, &["text", udid, text], limit).await?;

    Ok(())
}

pub async fn key(at: &Where, udid: &str, name: &str) -> Result<()> {
    check(udid)?;

    run(at, &["key", udid, button(name)?], Duration::from_secs(30)).await?;

    Ok(())
}

/// A `simctl` verb run for whether it worked rather than for what it printed.
/// The reason the far side gave is the whole answer where there is one; the
/// rest are for when it gave none, which is where a device that refused and a
/// session that dropped stop looking alike.
fn landed(at: &Where, what: &str, ran: ssh::Ran) -> Result<()> {
    let host = at.label();

    match ran.status {
        ssh::Status::Code(0) => Ok(()),
        ssh::Status::Code(_) if !ran.said.is_empty() => bail!("{}", ran.said),
        ssh::Status::Code(code) => bail!("{what} on {host} exited {code}"),
        ssh::Status::Garbled(said) => {
            bail!("{host} ended {what} with '{said}' rather than a status")
        }
        ssh::Status::Missing if ran.said.is_empty() => bail!("{host} did not run {what}"),
        ssh::Status::Missing => bail!("{}", ran.said),
    }
}

/// Boots `udid` and returns once the system is up, not once the process is.
/// `bootstatus` is what waits; `open` is only there so the window appears, and
/// a headless boot is still a usable device without it.
///
/// `limit` is a backstop rather than the deadline. The caller holds one of its
/// own and knows which device it is waiting on, so it is the one that should
/// answer — which only holds if what is passed here is strictly longer than it.
pub async fn boot(at: &Where, udid: &str, limit: Duration) -> Result<()> {
    // booting a device that is already up is an error worth ignoring, since
    // bootstatus answers for both cases and it is the one being asked
    const SCRIPT: &str = r#"xcrun simctl boot "$1" 2>/dev/null
open -a Simulator 2>/dev/null
xcrun simctl bootstatus "$1" -b >&2"#;

    check(udid)?;

    landed(
        at,
        "simctl bootstatus",
        at.exec(SCRIPT, &[udid], limit).await?,
    )
}

pub async fn shutdown(at: &Where, udid: &str) -> Result<()> {
    check(udid)?;

    let ran = at
        .exec(
            r#"exec xcrun simctl shutdown "$1""#,
            &[udid],
            Duration::from_secs(60),
        )
        .await?;

    if already_down(&ran.said) {
        return Ok(());
    }

    landed(at, "simctl shutdown", ran)
}

/// A device that is already down is the state that was asked for, and simctl
/// reports it as a failure to reach it. Now that its status is read at all, the
/// refusal has to be recognised or `stop` starts failing on a stopped device.
fn already_down(said: &str) -> bool {
    said.contains("current state: Shutdown")
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
    fn a_device_that_was_already_down_is_not_a_failed_shutdown() {
        assert!(already_down(
            "Unable to shutdown device in current state: Shutdown"
        ));
        assert!(!already_down(
            "Unable to shutdown device in current state: Booted"
        ));
        assert!(!already_down("Invalid device: nope"));
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

    fn shim(dir: &std::path::Path, name: &str) {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join(name);

        std::fs::write(&path, format!("#!/bin/sh\necho {name} \"$@\"\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn called(path: &std::path::Path) -> String {
        // absolute, and PATH holding nothing but the shims: a `sh` found through
        // PATH would not be found at all, and a wider PATH would let the real
        // binaries answer instead of the ones under test
        let out = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(CALL)
            .arg("sh")
            .arg("snapshot")
            .env("PATH", path)
            .output()
            .unwrap();

        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// The bug this closes: the bridge answered to `phone`, and `phone` on that
    /// host is now the CLI that calls it. Asking for it by name would have found
    /// the caller, and asking for the old name only would find the caller on
    /// every host that has been updated.
    #[test]
    fn the_bridge_is_asked_for_by_its_own_name_before_the_shared_one() {
        let dir = std::env::temp_dir().join(format!("phone-call-{}", std::process::id()));

        std::fs::create_dir_all(&dir).unwrap();

        // a host that has not been updated yet: the bridge is still `phone`
        shim(&dir, "phone");
        assert_eq!(called(&dir), "phone snapshot");

        // and one that has: `phone` is the CLI, so the bridge has to win
        shim(&dir, "phone-receiver");
        assert_eq!(called(&dir), "phone-receiver snapshot");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// iOS navigates back by a gesture or an on-screen control, so there is no
    /// button to press and nothing to quietly substitute.
    #[test]
    fn refuses_a_key_the_platform_does_not_have() {
        let err = button("back").unwrap_err().to_string();

        assert!(err.contains("no 'back' key"), "{err}");
        assert!(err.contains("home"), "the alternatives are listed: {err}");
    }
}
