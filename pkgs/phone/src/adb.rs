use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::process::Command;

use crate::model::Platform;

/// Which adb server a command talks to. adb has no notion of federation — a
/// server never speaks to another server — but the client picks its server per
/// invocation, so holding one forward per host and choosing between them here
/// gets the same result without either side knowing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Server {
    #[default]
    Local,
    Remote {
        host: String,
        port: u16,
    },
}

impl Server {
    pub fn host(&self) -> Option<&str> {
        match self {
            Server::Local => None,
            Server::Remote { host, .. } => Some(host),
        }
    }

    /// The client flag that redirects this invocation. Note it selects a
    /// *server*, so anything the client resolves for itself still happens
    /// locally — `adb emu` dials the emulator console on this machine's
    /// loopback and cannot reach a remote one.
    fn args(&self) -> Vec<String> {
        match self {
            Server::Local => Vec::new(),
            Server::Remote { port, .. } => {
                vec!["-L".to_string(), format!("tcp:127.0.0.1:{port}")]
            }
        }
    }

    pub fn command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("adb");
        cmd.args(self.args());

        cmd
    }

    /// The same redirection for tools that shell out to `adb` themselves and
    /// take no flags of ours — scrcpy being the one that matters.
    pub fn env(&self) -> Option<(&'static str, String)> {
        match self {
            Server::Local => None,
            Server::Remote { port, .. } => Some(("ANDROID_ADB_SERVER_PORT", port.to_string())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Attached {
    pub serial: String,
    pub state: String,
    pub model: String,
    pub product: String,
}

impl Attached {
    pub fn is_wireless(&self) -> bool {
        // a wireless transport is addressed as host:port; USB and emulator
        // serials never contain a colon.
        self.serial.contains(':')
    }

    pub fn platform(&self) -> Platform {
        if self
            .serial
            .starts_with(crate::model::EMULATOR_SERIAL_PREFIX)
        {
            Platform::Emulator
        } else {
            Platform::Android
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Identity {
    pub serialno: String,
    pub android_id: String,
    pub model: String,
}

impl Identity {
    /// Android 10+ hides `ro.serialno` from unprivileged callers on some
    /// vendors, so the settings-provider id is kept as a fallback key rather
    /// than letting the device fall back to its transient adb serial.
    pub fn best_id(&self) -> Option<String> {
        if !self.serialno.is_empty() {
            Some(self.serialno.clone())
        } else if !self.android_id.is_empty() {
            Some(format!("android_id:{}", self.android_id))
        } else {
            None
        }
    }
}

pub struct Output {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.status.success()
    }

    pub fn trimmed(&self) -> &str {
        self.stdout.trim()
    }
}

pub async fn run(server: &Server, args: &[&str]) -> Result<Output> {
    let out = Command::new("adb")
        .args(server.args())
        .args(args)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| anyhow!("running adb: {e}"))?;

    Ok(Output {
        status: out.status,
        // adb speaks CRLF whenever the device side ran under a pty
        stdout: String::from_utf8_lossy(&out.stdout).replace('\r', ""),
        stderr: String::from_utf8_lossy(&out.stderr).replace('\r', ""),
    })
}

pub async fn run_timeout(server: &Server, args: &[&str], limit: Duration) -> Result<Output> {
    tokio::time::timeout(limit, run(server, args))
        .await
        .map_err(|_| anyhow!("adb {} timed out", args.join(" ")))?
}

/// Raw bytes, for anything that is not text (`exec-out screencap -p`).
pub async fn run_bytes(server: &Server, args: &[&str]) -> Result<(bool, Vec<u8>)> {
    let out = Command::new("adb")
        .args(server.args())
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await?;

    Ok((out.status.success(), out.stdout))
}

pub async fn devices(server: &Server) -> Result<Vec<Attached>> {
    let out = run_timeout(server, &["devices", "-l"], Duration::from_secs(10)).await?;

    if !out.ok() {
        bail!("adb devices failed: {}", out.stderr.trim());
    }

    Ok(parse_devices(&out.stdout))
}

pub fn parse_devices(text: &str) -> Vec<Attached> {
    let mut out = Vec::new();

    for line in text.lines().skip(1) {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();

        let Some(serial) = parts.next() else { continue };
        let Some(state) = parts.next() else { continue };

        let mut dev = Attached {
            serial: serial.to_string(),
            state: state.to_string(),
            model: String::new(),
            product: String::new(),
        };

        for kv in parts {
            match kv.split_once(':') {
                Some(("model", v)) => dev.model = v.replace('_', " "),
                Some(("product", v)) => dev.product = v.to_string(),
                _ => {}
            }
        }

        out.push(dev);
    }

    out
}

pub async fn getprop(server: &Server, serial: &str, prop: &str) -> String {
    match run_timeout(
        server,
        &["-s", serial, "shell", "getprop", prop],
        Duration::from_secs(5),
    )
    .await
    {
        Ok(o) if o.ok() => o.trimmed().to_string(),
        _ => String::new(),
    }
}

pub async fn identity(server: &Server, serial: &str) -> Identity {
    let (serialno, boot_serialno, android_id, model) = tokio::join!(
        getprop(server, serial, "ro.serialno"),
        getprop(server, serial, "ro.boot.serialno"),
        async {
            match run_timeout(
                server,
                &[
                    "-s",
                    serial,
                    "shell",
                    "settings",
                    "get",
                    "secure",
                    "android_id",
                ],
                Duration::from_secs(5),
            )
            .await
            {
                Ok(o) if o.ok() && o.trimmed() != "null" => o.trimmed().to_string(),
                _ => String::new(),
            }
        },
        getprop(server, serial, "ro.product.model"),
    );

    Identity {
        serialno: if serialno.is_empty() {
            boot_serialno
        } else {
            serialno
        },
        android_id,
        model,
    }
}

/// AVD name. The `model` reported by `adb devices -l` is the system image and
/// identical across every emulator, so it cannot tell two apart.
///
/// Read as a property rather than over `adb emu`: that command is resolved by
/// the *client*, which derives the console port from `emulator-5554` and dials
/// it on its own loopback, so through a forwarded server it lands on this
/// machine where nothing is listening. A property rides the transport and works
/// the same whichever server answered.
pub async fn avd_name(server: &Server, serial: &str) -> String {
    let name = getprop(server, serial, "ro.boot.qemu.avd_name").await;

    if !name.is_empty() {
        return name;
    }

    getprop(server, serial, "ro.kernel.qemu.avd_name").await
}

pub async fn connect(server: &Server, addr: &str) -> Result<()> {
    let out = run_timeout(server, &["connect", addr], Duration::from_secs(12)).await?;
    let text = format!("{}{}", out.stdout, out.stderr);

    // `adb connect` exits 0 on refusal and reports it on stdout instead
    if text.contains("connected to") {
        Ok(())
    } else {
        bail!("{}", text.trim().lines().next().unwrap_or("connect failed"))
    }
}

pub async fn disconnect(server: &Server, addr: &str) -> Result<()> {
    run_timeout(server, &["disconnect", addr], Duration::from_secs(8)).await?;

    Ok(())
}

pub async fn tcpip(server: &Server, serial: &str, port: u16) -> Result<()> {
    let out = run_timeout(
        server,
        &["-s", serial, "tcpip", &port.to_string()],
        Duration::from_secs(12),
    )
    .await?;

    if out.ok() {
        Ok(())
    } else {
        bail!("{}", out.stderr.trim())
    }
}

pub async fn pair(server: &Server, addr: &str, code: &str) -> Result<()> {
    let out = run_timeout(server, &["pair", addr, code], Duration::from_secs(30)).await?;
    let text = format!("{}{}", out.stdout, out.stderr);

    if text.contains("Successfully paired") {
        Ok(())
    } else {
        bail!("{}", text.trim().lines().next().unwrap_or("pairing failed"))
    }
}

/// A panel, in both of the id namespaces Android keeps for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    /// SurfaceFlinger's physical id, which is what `screencap -d` takes. 64 bit
    /// and on current hardware always past what a `u32` holds.
    pub physical: u64,
    /// The logical id, which is what `input -d` takes. Unrelated to the
    /// physical one: a fold numbers its two panels 0 and 3 where SurfaceFlinger
    /// numbers them 4619827677550801152 and ...153, and each command rejects
    /// the other's id outright.
    pub logical: u32,
}

/// The panel that is live. Foldables expose several and both `screencap` and
/// `input` otherwise fall to the first, which is as often as not the one that
/// is closed.
pub async fn active_display(server: &Server, serial: &str) -> Option<Display> {
    let out = run_timeout(
        server,
        &["-s", serial, "shell", "dumpsys", "display"],
        Duration::from_secs(8),
    )
    .await
    .ok()?;

    active_viewport(&out.stdout)
}

/// The viewport `dumpsys display` marks active.
///
/// Read from there rather than from SurfaceFlinger's `powerMode` because
/// `isActive` is the input system's own record of which panel is live — the
/// question being asked — and because SurfaceFlinger never names the logical id
/// at all.
pub fn active_viewport(text: &str) -> Option<Display> {
    let record = text
        .split("DisplayViewport{")
        .skip(1)
        .map(|record| &record[..record.find('}').unwrap_or(record.len())])
        .find(|record| field(record, "isActive=") == Some("true"))?;

    Some(Display {
        physical: field(record, "uniqueId='local:")?.parse().ok()?,
        logical: field(record, "displayId=")?.parse().ok()?,
    })
}

/// The value written after `key`, up to whatever closes it.
fn field<'a>(record: &'a str, key: &str) -> Option<&'a str> {
    let rest = record.split_once(key)?.1;

    Some(&rest[..rest.find([',', '\'', '}']).unwrap_or(rest.len())])
}

pub async fn pidof(server: &Server, serial: &str, package: &str) -> Option<String> {
    let out = run_timeout(
        server,
        &["-s", serial, "shell", "pidof", package],
        Duration::from_secs(6),
    )
    .await
    .ok()?;

    out.trimmed().split_whitespace().next().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Pixel 10 Pro Fold, open, whose two panels are the reason any of this
    /// exists. Trimmed of the geometry, which is not read.
    const FOLD: &str = "  mViewports=[DisplayViewport{type=INTERNAL, valid=true, isActive=true, \
displayId=0, uniqueId='local:4619827677550801152', physicalPort=0, orientation=0, \
deviceWidth=2076, deviceHeight=2152}, DisplayViewport{type=INTERNAL, valid=true, \
isActive=false, displayId=3, uniqueId='local:4619827677550801153', physicalPort=1, \
orientation=0, deviceWidth=1080, deviceHeight=2364}]
  mDefaultDisplayDefaultColorMode=0";

    #[test]
    fn pairs_the_live_panel_with_both_its_ids() {
        assert_eq!(
            active_viewport(FOLD),
            Some(Display {
                physical: 4619827677550801152,
                logical: 0,
            })
        );
    }

    /// Folded, the second viewport takes over. Nothing but `isActive` moves, so
    /// reading position instead of that flag would answer the same either way.
    #[test]
    fn follows_the_flag_rather_than_the_order() {
        let folded = FOLD.replacen("isActive=true", "isActive=x", 1);
        let folded = folded.replace("isActive=false", "isActive=true");

        assert_eq!(
            active_viewport(&folded),
            Some(Display {
                physical: 4619827677550801153,
                logical: 3,
            })
        );
    }

    #[test]
    fn answers_nothing_when_no_viewport_is_live() {
        assert_eq!(
            active_viewport(&FOLD.replace("isActive=true", "isActive=false")),
            None
        );
    }
}
