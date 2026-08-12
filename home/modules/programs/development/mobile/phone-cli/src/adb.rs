use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::process::Command;

use crate::model::Platform;

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
        if self.serial.starts_with("emulator-") {
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

pub async fn run(args: &[&str]) -> Result<Output> {
    let out = Command::new("adb")
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

pub async fn run_timeout(args: &[&str], limit: Duration) -> Result<Output> {
    tokio::time::timeout(limit, run(args))
        .await
        .map_err(|_| anyhow!("adb {} timed out", args.join(" ")))?
}

/// Raw bytes, for anything that is not text (`exec-out screencap -p`).
pub async fn run_bytes(args: &[&str]) -> Result<(bool, Vec<u8>)> {
    let out = Command::new("adb")
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await?;

    Ok((out.status.success(), out.stdout))
}

pub async fn devices() -> Result<Vec<Attached>> {
    let out = run_timeout(&["devices", "-l"], Duration::from_secs(10)).await?;

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

pub async fn getprop(serial: &str, prop: &str) -> String {
    match run_timeout(
        &["-s", serial, "shell", "getprop", prop],
        Duration::from_secs(5),
    )
    .await
    {
        Ok(o) if o.ok() => o.trimmed().to_string(),
        _ => String::new(),
    }
}

pub async fn identity(serial: &str) -> Identity {
    let (serialno, boot_serialno, android_id, model) = tokio::join!(
        getprop(serial, "ro.serialno"),
        getprop(serial, "ro.boot.serialno"),
        async {
            match run_timeout(
                &["-s", serial, "shell", "settings", "get", "secure", "android_id"],
                Duration::from_secs(5),
            )
            .await
            {
                Ok(o) if o.ok() && o.trimmed() != "null" => o.trimmed().to_string(),
                _ => String::new(),
            }
        },
        getprop(serial, "ro.product.model"),
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

/// AVD console name. The `model` reported by `adb devices -l` is the system
/// image and identical across every emulator, so it cannot tell two apart.
pub async fn avd_name(serial: &str) -> String {
    match run_timeout(&["-s", serial, "emu", "avd", "name"], Duration::from_secs(5)).await {
        Ok(o) if o.ok() => o
            .stdout
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
        _ => String::new(),
    }
}

pub async fn connect(addr: &str) -> Result<()> {
    let out = run_timeout(&["connect", addr], Duration::from_secs(12)).await?;
    let text = format!("{}{}", out.stdout, out.stderr);

    // `adb connect` exits 0 on refusal and reports it on stdout instead
    if text.contains("connected to") {
        Ok(())
    } else {
        bail!("{}", text.trim().lines().next().unwrap_or("connect failed"))
    }
}

pub async fn disconnect(addr: &str) -> Result<()> {
    run_timeout(&["disconnect", addr], Duration::from_secs(8)).await?;

    Ok(())
}

pub async fn tcpip(serial: &str, port: u16) -> Result<()> {
    let out = run_timeout(
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

pub async fn pair(addr: &str, code: &str) -> Result<()> {
    let out = run_timeout(&["pair", addr, code], Duration::from_secs(30)).await?;
    let text = format!("{}{}", out.stdout, out.stderr);

    if text.contains("Successfully paired") {
        Ok(())
    } else {
        bail!("{}", text.trim().lines().next().unwrap_or("pairing failed"))
    }
}

/// The display whose panel is actually on. Foldables expose several internal
/// displays and `screencap` defaults to "the first one found", which is usually
/// the one that is off.
pub async fn active_display(serial: &str) -> Option<u32> {
    let out = run_timeout(
        &["-s", serial, "shell", "dumpsys", "SurfaceFlinger", "--displays"],
        Duration::from_secs(8),
    )
    .await
    .ok()?;

    let mut current = None;

    for line in out.stdout.lines() {
        let line = line.trim_end();

        if let Some(rest) = line.strip_prefix("Display ") {
            if let Ok(id) = rest.trim().parse::<u32>() {
                current = Some(id);
            }
        }

        if line.ends_with("powerMode=On") {
            return current;
        }
    }

    None
}

pub async fn pidof(serial: &str, package: &str) -> Option<String> {
    let out = run_timeout(
        &["-s", serial, "shell", "pidof", package],
        Duration::from_secs(6),
    )
    .await
    .ok()?;

    out.trimmed().split_whitespace().next().map(str::to_string)
}
