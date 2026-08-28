use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::process::Command;

use crate::model::Platform;
use crate::ssh;

/// Which adb server a command talks to. Servers never speak to each other, but
/// the client picks its server per invocation, so one forward per host and a
/// choice here gets the same result.
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

    /// Selects a *server*, so anything the client resolves for itself still
    /// happens locally — `adb emu` dials the console on this machine's loopback.
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

    /// The same redirection for tools that shell out to `adb` themselves.
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
        // USB and emulator serials never contain a colon
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
    /// The name the emulator was started under. Empty on anything else.
    pub avd: String,
}

/// What an emulator answers `ro.serialno` with. It is minted from the system
/// image rather than the instance, so every emulator started from one image
/// shares it and no handset serial has the shape.
pub const EMULATOR_BUILD_SERIAL: &str = "EMULATOR";

/// `adb shell` merges what a command prints with what it complains about, so a
/// settings read that fails comes back looking exactly like an answer: the id
/// becomes `cmd: Failure calling service settings: ...`. Every real one is hex,
/// which is enough to tell the two apart — and without the check an emulator
/// earns a second, permanent registry entry under the error text, which then
/// makes its own name ambiguous.
fn is_settings_id(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_hexdigit())
}

impl Identity {
    fn parse(out: &str) -> Self {
        let mut ident = Identity::default();
        let mut boot_serialno = String::new();
        let mut avd_kernel = String::new();

        for line in out.lines() {
            let Some((key, value)) = line.trim().split_once('=') else {
                continue;
            };

            let value = value.trim().to_string();

            match key {
                "serialno" => ident.serialno = value,
                "boot_serialno" => boot_serialno = value,
                "model" => ident.model = value,
                "avd" => ident.avd = value,
                "avd_kernel" => avd_kernel = value,
                "android_id" if is_settings_id(&value) => ident.android_id = value,
                _ => {}
            }
        }

        if ident.serialno.is_empty() {
            ident.serialno = boot_serialno;
        }

        if ident.avd.is_empty() {
            ident.avd = avd_kernel;
        }

        ident
    }

    pub fn is_emulator(&self) -> bool {
        self.serialno.starts_with(EMULATOR_BUILD_SERIAL)
    }

    /// Android 10+ hides `ro.serialno` from unprivileged callers on some
    /// vendors; the settings-provider id is the fallback key. For an emulator it
    /// is the only key, `ro.serialno` being the same for all of them.
    pub fn best_id(&self) -> Option<String> {
        let settings_id =
            || (!self.android_id.is_empty()).then(|| format!("android_id:{}", self.android_id));

        if self.is_emulator() {
            return settings_id();
        }

        if !self.serialno.is_empty() {
            return Some(self.serialno.clone());
        }

        settings_id()
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

/// `adb` lives inside the SDK rather than in any system path, and a
/// non-interactive ssh reads no login profile, so the login shell is asked for
/// its own PATH the way host probing is.
const REMOTE_ADB: &str = r#"PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
exec adb "$@""#;

/// A read that only asks a host what it already knows, run on the host itself
/// rather than through its forwarded adb server.
///
/// adb's protocol takes several round trips per command, and each of those
/// crosses the link to the host; one ssh session crosses it once. Against a
/// host a few hundred milliseconds away that is the difference between a survey
/// that takes seconds and one that does not. Only reads go this way: `install`
/// sends a file that exists on this machine, and a screenshot comes back as
/// bytes, so both still want the forward.
async fn read(server: &Server, args: &[&str], limit: Duration) -> Result<Output> {
    let Server::Remote { host, .. } = server else {
        return run_timeout(server, args, limit).await;
    };

    let out = tokio::time::timeout(limit, ssh::script(host, REMOTE_ADB, args).output())
        .await
        .map_err(|_| anyhow!("adb {} timed out on {host}", args.join(" ")))?
        .map_err(|e| anyhow!("running adb on {host}: {e}"))?;

    Ok(Output {
        status: out.status,
        stdout: String::from_utf8_lossy(&out.stdout).replace('\r', ""),
        stderr: String::from_utf8_lossy(&out.stderr).replace('\r', ""),
    })
}

pub async fn devices(server: &Server) -> Result<Vec<Attached>> {
    let out = read(server, &["devices", "-l"], Duration::from_secs(15)).await?;

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

/// Everything that names a device, asked for in one device-side shell. Each
/// `adb shell` is a round trip, and against a host reached over the network
/// that costs far more than the properties themselves, so a survey that asked
/// for them one at a time spent seconds waiting rather than reading.
///
/// `ro.kernel.qemu.avd_name` is the older spelling of the AVD name, and
/// `settings get` answers the literal `null` when the row is unset.
const IDENTITY: &str = concat!(
    r#"echo "serialno=$(getprop ro.serialno)";"#,
    r#"echo "boot_serialno=$(getprop ro.boot.serialno)";"#,
    r#"echo "model=$(getprop ro.product.model)";"#,
    r#"echo "avd=$(getprop ro.boot.qemu.avd_name)";"#,
    r#"echo "avd_kernel=$(getprop ro.kernel.qemu.avd_name)";"#,
    r#"echo "android_id=$(settings get secure android_id)""#,
);

pub async fn identity(server: &Server, serial: &str) -> Identity {
    let out = match read(
        server,
        &["-s", serial, "shell", IDENTITY],
        Duration::from_secs(15),
    )
    .await
    {
        Ok(o) if o.ok() => o.stdout,
        _ => return Identity::default(),
    };

    Identity::parse(&out)
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

/// A panel, in both of the id namespaces Android keeps for it. A fold numbers
/// its two panels 0 and 3 where SurfaceFlinger numbers them
/// 4619827677550801152 and ...153, and each command rejects the other's id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    /// What `screencap -d` takes.
    pub physical: u64,
    /// What `input -d` takes.
    pub logical: u32,
}

/// The panel that is live. Foldables expose several and both `screencap` and
/// `input` otherwise fall to the first, as often as not the one that is closed.
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

/// Read here rather than from SurfaceFlinger's `powerMode`: `isActive` is the
/// input system's own record of which panel is live, and SurfaceFlinger never
/// names the logical id at all.
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

fn field<'a>(record: &'a str, key: &str) -> Option<&'a str> {
    let rest = record.split_once(key)?.1;

    Some(&rest[..rest.find([',', '\'', '}']).unwrap_or(rest.len())])
}

/// The panel in pixels. Element bounds and taps are in pixels on Android, so
/// this is the one space there — unlike a simulator, which reports points.
/// `wm size` answers for the default display unless the panel is named, which
/// matters on a fold, where the closed and open panels differ.
pub async fn screen_size(
    server: &Server,
    serial: &str,
    display: Option<Display>,
) -> Option<(i32, i32)> {
    let aim = display
        .map(|d| format!(" -d {}", d.logical))
        .unwrap_or_default();
    let out = run_timeout(
        server,
        &["-s", serial, "shell", &format!("wm size{aim}")],
        Duration::from_secs(8),
    )
    .await
    .ok()?;

    parse_wm_size(&out.stdout)
}

/// `wm size` prints the physical size and, when something has overridden it, an
/// override line as well. The override is what everything else reports.
pub fn parse_wm_size(text: &str) -> Option<(i32, i32)> {
    let mut found = None;

    for line in text.lines() {
        let Some((_, dims)) = line.rsplit_once(": ") else {
            continue;
        };

        let Some((w, h)) = dims.trim().split_once('x') else {
            continue;
        };

        let dims = (w.trim().parse().ok()?, h.trim().parse().ok()?);

        if line.starts_with("Physical size") {
            found = Some(dims);
        } else if line.starts_with("Override size") {
            return Some(dims);
        }
    }

    found
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

    /// A Pixel 10 Pro Fold, open, trimmed of the geometry that is not read.
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

    /// Folded, the second viewport takes over; nothing but `isActive` moves.
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

    /// The failure this actually produced: a broken pipe became the emulator's
    /// id, and the registry then held two devices answering to `pixel_7-api36`,
    /// so naming it was ambiguous and every command against it stopped.
    #[test]
    fn a_settings_read_that_failed_is_not_mistaken_for_an_id() {
        let broken = Identity::parse(
            "serialno=EMULATOR36X6X11X0\r\nboot_serialno=EMULATOR36X6X11X0\r\n\
             model=sdk_gphone64_arm64\r\navd=pixel_7-api36\r\n\
             android_id=cmd: Failure calling service settings: Broken pipe (32)\r\n",
        );

        assert!(broken.android_id.is_empty());
        assert_eq!(broken.best_id(), None, "no id at all beats a wrong one");
    }

    /// The one shell answers for every device, so a handset leaves the emulator
    /// keys blank and an older emulator answers under the kernel spelling.
    #[test]
    fn one_shell_answers_for_every_kind_of_device() {
        let emu = Identity::parse(
            "serialno=EMULATOR36X6X11X0\r\nboot_serialno=EMULATOR36X6X11X0\r\n\
             model=sdk_gphone64_arm64\r\navd=\r\navd_kernel=pixel_7-api36\r\n\
             android_id=6a2c0a1c04bc476d\r\n",
        );

        assert_eq!(emu.serialno, "EMULATOR36X6X11X0");
        assert_eq!(
            emu.avd, "pixel_7-api36",
            "falls back to the kernel spelling"
        );
        assert_eq!(emu.android_id, "6a2c0a1c04bc476d");
        assert!(emu.is_emulator());

        let handset = Identity::parse(
            "serialno=\nboot_serialno=58281FDCG001K5\nmodel=Pixel 10 Pro Fold\n\
             avd=\navd_kernel=\nandroid_id=null\n",
        );

        assert_eq!(
            handset.serialno, "58281FDCG001K5",
            "a vendor that hides ro.serialno still boots with one"
        );
        assert_eq!(handset.model, "Pixel 10 Pro Fold");
        assert!(handset.avd.is_empty());
        assert!(
            handset.android_id.is_empty(),
            "an unset settings row reads as the literal null"
        );
    }

    #[test]
    fn an_emulator_is_keyed_by_the_only_id_that_varies_per_instance() {
        let emu = Identity {
            serialno: "EMULATOR36X6X11X0".into(),
            android_id: "29a1ed706918672b".into(),
            model: "sdk gphone64 arm64".into(),
            ..Default::default()
        };
        let handset = Identity {
            serialno: "58281FDCG001K5".into(),
            android_id: "28aeb91fdbf85825".into(),
            model: "Pixel 10 Pro Fold".into(),
            ..Default::default()
        };

        assert_eq!(
            emu.best_id().as_deref(),
            Some("android_id:29a1ed706918672b"),
            "every emulator from one image shares ro.serialno"
        );
        assert_eq!(handset.best_id().as_deref(), Some("58281FDCG001K5"));
    }

    #[test]
    fn an_emulator_with_nothing_to_key_on_is_unidentified() {
        let emu = Identity {
            serialno: "EMULATOR36X6X11X0".into(),
            ..Identity::default()
        };

        assert_eq!(emu.best_id(), None);
    }

    #[test]
    fn an_override_is_what_the_rest_of_the_system_reports() {
        assert_eq!(
            parse_wm_size("Physical size: 1080x2400\n"),
            Some((1080, 2400))
        );
        assert_eq!(
            parse_wm_size("Physical size: 1440x3120\nOverride size: 1080x2340\n"),
            Some((1080, 2340))
        );
        assert_eq!(parse_wm_size("mumble\n"), None);
    }

    #[test]
    fn answers_nothing_when_no_viewport_is_live() {
        assert_eq!(
            active_viewport(&FOLD.replace("isActive=true", "isActive=false")),
            None
        );
    }
}
