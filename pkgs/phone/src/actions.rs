use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};

use crate::a11y::Bounds;
use crate::adb::{self, Server};
use crate::connect::{attached_serial, Reporter};
use crate::model::{Device, Platform, Reach};
use crate::ssh::Where;
use crate::{avd, ios, simctl};

const PNG_MAGIC: [u8; 4] = [0x89, b'P', b'N', b'G'];

/// Long enough for a cold emulator on a busy host. The CLI takes `--timeout`;
/// the browser has no way to pass one.
pub const BOOT_TIMEOUT: Duration = Duration::from_secs(180);

/// Where a device is driven from: the machine it hangs off, or this one when it
/// hangs off nothing. A simulator on the mac reads as `Here` to a `phone`
/// running on that mac, and as `On("rose")` to one running anywhere else.
pub fn where_of(device: &Device) -> Where {
    Where::of(device.host.as_deref())
}

/// The machine a hosted device hangs off. An error rather than a fallback to
/// this machine, which would silently drive the wrong device — which is right
/// for the paths that can only reach a device over the network, and wrong for
/// the ones that also work on the machine holding it. Those take `where_of`.
pub fn host_of(device: &Device) -> Result<&str> {
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

/// What to do to a frame before it is handed over. A full one costs the reader
/// far more than the part of it that was being looked at, and most looks are
/// aimed at one control.
#[derive(Clone, Copy, Debug, Default)]
pub struct Shot {
    /// In pixels, which is not the space element bounds come in everywhere.
    pub crop: Option<Bounds>,
    pub scale: Option<f64>,
    /// JPEG quality, for a frame whose exact pixels do not matter.
    pub jpeg: Option<u8>,
    /// Capture until two frames running are identical, so a tap's result is
    /// read rather than the frame that was still on screen when it landed.
    pub settle: bool,
}

impl Shot {
    fn touches_the_image(&self) -> bool {
        self.crop.is_some() || self.scale.is_some() || self.jpeg.is_some()
    }

    fn mime(&self) -> &'static str {
        match self.jpeg {
            Some(_) => "image/jpeg",
            None => "image/png",
        }
    }
}

/// How long a screen is given to stop changing. Past this the frames are handed
/// over anyway: a video, a spinner or a caret blinking never settles.
const SETTLE_LIMIT: Duration = Duration::from_secs(6);
const SETTLE_STEP: Duration = Duration::from_millis(350);

/// `screencap` and pymobiledevice3 both exit 0 on some failures, so a capture
/// only counts once real PNG bytes come back.
pub async fn screenshot(
    server: &Server,
    device: &Device,
    sink: &Sink,
    rep: &Reporter,
    shot: &Shot,
) -> Result<String> {
    let mut png = capture(server, device, rep).await?;

    if shot.settle {
        png = settle(server, device, rep, png).await?;
    }

    let png = render(png, shot)?;

    deliver(device, sink, png, shot).await
}

/// Frames until two running are the same. Compared whole rather than by a hash
/// of part of them: a PNG of an unchanged screen is byte-identical, and any
/// cheaper comparison would have to guess where the change would be.
async fn settle(
    server: &Server,
    device: &Device,
    rep: &Reporter,
    first: Vec<u8>,
) -> Result<Vec<u8>> {
    let started = std::time::Instant::now();
    let mut previous = first;

    while started.elapsed() < SETTLE_LIMIT {
        tokio::time::sleep(SETTLE_STEP).await;

        let next = capture(server, device, rep).await?;

        if next == previous {
            return Ok(next);
        }

        previous = next;
    }

    rep.note("the screen never stopped changing");

    Ok(previous)
}

pub fn render(png: Vec<u8>, shot: &Shot) -> Result<Vec<u8>> {
    // re-encoding a frame nothing was asked of would only cost it quality
    if !shot.touches_the_image() {
        return Ok(png);
    }

    let mut image = image::load_from_memory(&png).context("decoding the frame")?;

    if let Some(bounds) = shot.crop {
        let (w, h) = (image.width(), image.height());
        let x = bounds.x1.max(0) as u32;
        let y = bounds.y1.max(0) as u32;

        if x >= w || y >= h {
            bail!("the crop starts outside a {w}x{h} frame");
        }

        image = image.crop_imm(
            x,
            y,
            (bounds.width() as u32).min(w - x).max(1),
            (bounds.height() as u32).min(h - y).max(1),
        );
    }

    if let Some(by) = shot.scale {
        let at = |v: u32| ((f64::from(v) * by).round() as u32).max(1);

        image = image.resize_exact(
            at(image.width()),
            at(image.height()),
            image::imageops::FilterType::Lanczos3,
        );
    }

    let mut out = std::io::Cursor::new(Vec::new());

    match shot.jpeg {
        // JPEG has no alpha channel and the encoder refuses a buffer with one
        Some(quality) => image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
            .encode_image(&image.to_rgb8())?,
        None => image.write_to(&mut out, image::ImageFormat::Png)?,
    }

    Ok(out.into_inner())
}

async fn capture(server: &Server, device: &Device, rep: &Reporter) -> Result<Vec<u8>> {
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
        Platform::Simulator => {
            simctl::screenshot(&where_of(device), simctl::udid(device)?).await?
        }
        _ => {
            let serial = attached_serial(server, device)
                .await
                .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

            let display = adb::active_display(server, &serial).await;

            // exec-out keeps CRLF translation off the PNG but folds stderr
            // into it, so the redirect has to run device-side
            let remote = match display {
                Some(d) => format!("screencap -p -d {} 2>/dev/null", d.physical),
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

    Ok(png)
}

async fn deliver(device: &Device, sink: &Sink, png: Vec<u8>, shot: &Shot) -> Result<String> {
    let png = &png[..];

    let (msg, body, icon) = match sink {
        Sink::Stdout => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(png)?;
            stdout.flush()?;

            // the caller is piping the bytes somewhere, so a popup is noise
            return Ok("wrote PNG to stdout".into());
        }
        Sink::File(path) => {
            std::fs::write(path, png).with_context(|| format!("writing {path}"))?;

            let msg = format!("saved {path}");

            (msg.clone(), msg, preview(png))
        }
        Sink::Clipboard => {
            let mut cmd = tokio::process::Command::new("wl-copy");

            cmd.args(["--type", shot.mime()]);

            // wl-copy forks a daemon that keeps serving the selection, so the
            // image only lives as long as that process does. Leaving it in this
            // process group means closing the terminal SIGHUPs the clipboard
            // contents away along with the TUI.
            cmd.process_group(0);

            pipe_to(&mut cmd, png, "wl-copy").await?;

            (
                format!("copied {} to the clipboard", device.label),
                "copied to the clipboard".to_string(),
                shot.jpeg.is_none().then(|| preview(png)).flatten(),
            )
        }
    };

    notify(&device.label, &body, icon.as_deref()).await;

    Ok(msg)
}

/// A notification daemon decodes an icon at its full size before scaling it for
/// display, and holds that buffer for as long as the notification stays in its
/// history. A phone frame is ~12 MB decoded, so what goes on the bus is a
/// thumbnail rather than the capture itself.
const PREVIEW_EDGE: u32 = 256;

/// A notification daemon reads its icon off disk, so a capture still needs a
/// file even when it went somewhere else. One fixed name under the runtime dir,
/// overwritten each time and gone with the session.
fn preview(png: &[u8]) -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    preview_in(&dir.join("phone"), &thumbnail(png)?)
}

/// None when the bytes will not decode, which drops the icon rather than
/// handing the daemon a frame it would hold at full size.
fn thumbnail(png: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(png).ok()?;

    let mut out = std::io::Cursor::new(Vec::new());

    image
        .thumbnail(PREVIEW_EDGE, PREVIEW_EDGE)
        .write_to(&mut out, image::ImageFormat::Png)
        .ok()?;

    Some(out.into_inner())
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

/// Only the front process is waited on: anything it forks inherits the stderr
/// pipe and can outlive it, so that pipe is read on failure alone.
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
    let app = crate::apps::app_id(app)?;

    match device.platform {
        Platform::Ios => return ios::logs_command(host_of(device)?, ios::udid(device)?, app),
        Platform::Simulator => {
            return simctl::logs_command(&where_of(device), simctl::udid(device)?, app)
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

/// What to ask `adb reverse` for.
#[derive(Clone, Copy, Debug)]
pub enum Reverse {
    Open { device: u16, host: u16 },
    List,
    Clear,
}

/// Long enough for a wireless device on the far side of the tailnet, short
/// enough that a device that has gone away is reported rather than waited on.
const REVERSE_TIMEOUT: Duration = Duration::from_secs(10);

/// A port on the device that answers on the adb server's machine.
///
/// Which machine that is, is the whole subtlety, and it is not this one: the
/// forward terminates wherever the adb server holding the device runs. For an
/// emulator on a mac, `phone device reverse 8081` points the device at that mac's
/// loopback, which is where a Metro started in an ssh session there is
/// listening. A bundler running here is not what the device will reach.
pub async fn reverse(server: &Server, device: &Device, what: Reverse) -> Result<String> {
    if device.platform == Platform::Simulator {
        bail!("a simulator already shares its host's loopback, so there is nothing to reverse");
    }

    if device.platform.is_hosted() {
        bail!(
            "{} has no adb transport to reverse a port over",
            device.platform
        );
    }

    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    let verb: Vec<String> = match what {
        Reverse::Open { device, host } => {
            vec![format!("tcp:{device}"), format!("tcp:{host}")]
        }
        Reverse::List => vec!["--list".to_string()],
        Reverse::Clear => vec!["--remove-all".to_string()],
    };

    let mut args = vec!["-s", &serial, "reverse"];
    args.extend(verb.iter().map(String::as_str));

    let out = adb::run_timeout(server, &args, REVERSE_TIMEOUT).await?;

    if !out.ok() {
        // adb puts the reason on stderr and says nothing on stdout, and the
        // usual reason is a port already claimed by another forward
        bail!(
            "{}",
            first_line(&out.stderr).unwrap_or("adb refused the reverse")
        );
    }

    Ok(match what {
        Reverse::Open { device: d, host } => {
            let at = server.host().unwrap_or("this machine");

            format!("{}:{d} now reaches {at}:{host}", device.label)
        }
        // adb prints one `<serial> tcp:8081 tcp:8081` line per forward, whose
        // first column is the transport name this program made up and nothing
        // here answers to; and prints nothing at all when there are none, which
        // reads as a failure unless it is said out loud
        Reverse::List => match forwards(out.trimmed(), server.host().unwrap_or("this machine")) {
            listed if listed.is_empty() => format!("{} has no reverse forwards", device.label),
            listed => listed.join("\n"),
        },
        Reverse::Clear => format!("removed every reverse forward on {}", device.label),
    })
}

/// The reverse forwards a device already holds, as `(device, host)` port pairs.
///
/// `reverse` formats them for a reader; converging on them needs the numbers.
/// Opening one that is already open is harmless, but asking first is what lets
/// `up` say a device was already ready instead of reporting work it did not do.
pub async fn reversed(server: &Server, device: &Device) -> Result<Vec<(u16, u16)>> {
    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    let out = adb::run_timeout(
        server,
        &["-s", &serial, "reverse", "--list"],
        REVERSE_TIMEOUT,
    )
    .await?;

    if !out.ok() {
        bail!(
            "{}",
            first_line(&out.stderr).unwrap_or("adb refused to list the reverses")
        );
    }

    let port = |raw: &str| raw.trim_start_matches("tcp:").parse::<u16>().ok();

    Ok(out
        .trimmed()
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>()[..] {
                [_, from, to] => Some((port(from)?, port(to)?)),
                _ => None,
            },
        )
        .collect())
}

/// The namespaces `settings` divides a device's configuration into. Anything
/// else is a typo, and interpolating it into the device-side command would be
/// the injection this check exists to stop.
fn namespace(ns: &str) -> Result<&str> {
    match ns {
        "global" | "system" | "secure" => Ok(ns),
        other => bail!("{other} is not a settings namespace (global, system, secure)"),
    }
}

/// A key or a value, on its way into a shell command on the device.
fn word(raw: &str, what: &str) -> Result<()> {
    if raw.is_empty()
        || !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        bail!("invalid setting {what}: {raw}");
    }

    Ok(())
}

/// Brings a device's settings to what was asked for, and reports only what it
/// had to change.
///
/// Every one is read before any is written, in one round trip, because the
/// common case is that they already match: an emulator that was left as it was
/// converges without touching the device at all. The reads and the writes are
/// each one `adb shell`, since a per-setting call would be a round trip to the
/// host holding the device for each.
///
/// These are the animation scales more than anything else. An emulator created
/// from a headless image has them at 0, and a UI test driven against that reads
/// screens that are still mid-transition.
pub async fn settings(
    server: &Server,
    device: &Device,
    want: &[(&str, &str, String)],
) -> Result<Vec<String>> {
    if want.is_empty() {
        return Ok(Vec::new());
    }

    if !device.platform.is_adb() {
        bail!(
            "{} takes its settings from its host, not from adb",
            device.platform
        );
    }

    for (ns, key, value) in want {
        namespace(ns)?;
        word(key, "key")?;
        word(value, "value")?;
    }

    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    let reads: Vec<String> = want
        .iter()
        .map(|(ns, key, _)| format!("settings get {ns} {key}"))
        .collect();

    let out = adb::run_timeout(
        server,
        &["-s", &serial, "shell", &reads.join("\n")],
        SETTINGS_TIMEOUT,
    )
    .await?;

    if !out.ok() {
        bail!(
            "{}",
            first_line(&out.stderr).unwrap_or("adb refused to read the settings")
        );
    }

    // an unset setting answers `null`, which is a value nothing will ever be
    // asked for and so never matches
    let have: Vec<&str> = out.stdout.lines().map(str::trim).collect();

    let stale: Vec<&(&str, &str, String)> = want
        .iter()
        .enumerate()
        .filter(|(i, (_, _, value))| have.get(*i).copied() != Some(value.as_str()))
        .map(|(_, spec)| spec)
        .collect();

    if stale.is_empty() {
        return Ok(Vec::new());
    }

    let writes: Vec<String> = stale
        .iter()
        .map(|(ns, key, value)| format!("settings put {ns} {key} {value}"))
        .collect();

    let out = adb::run_timeout(
        server,
        &["-s", &serial, "shell", &writes.join("\n")],
        SETTINGS_TIMEOUT,
    )
    .await?;

    if !out.ok() {
        bail!(
            "{}",
            first_line(&out.stderr).unwrap_or("adb refused to write the settings")
        );
    }

    Ok(stale
        .iter()
        .map(|(ns, key, value)| format!("{ns}.{key} = {value}"))
        .collect())
}

/// Short: `settings` is a local database read, and a device that cannot answer
/// it in this long is not going to.
const SETTINGS_TIMEOUT: Duration = Duration::from_secs(15);

fn first_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|l| !l.is_empty())
}

/// `adb reverse --list` answers with the transport name this program made up
/// and two `tcp:` pairs. Neither column is something the reader can type back,
/// so it is rewritten into the shape `reverse` takes and the host it lands on.
fn forwards(stdout: &str, at: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>()[..] {
                [_, from, to] => Some(format!(
                    "{} -> {at}:{}",
                    from.trim_start_matches("tcp:"),
                    to.trim_start_matches("tcp:")
                )),
                _ => None,
            },
        )
        .collect()
}

/// Detached, because scrcpy owns a window rather than the terminal. Works
/// against a remote server unchanged: the encoder runs on the device and the
/// stream rides the same adb connection.
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
        simctl::install(&where_of(device), simctl::udid(device)?, bundle).await?;

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

/// Whether a device is up, which each platform says differently: an emulator
/// holds an adb transport, a simulator only ever shows as listed on its host.
pub fn running(reach: &Reach) -> bool {
    reach.is_attached() || *reach == Reach::Online
}

/// Starts a device and returns only once it can be driven, which is a later
/// moment than the one the process starts at: a simulator answers `boot`
/// immediately and an emulator shows its window well before the system is up.
/// Returning any earlier hands back a device that fails the first tap.
pub async fn boot(device: &Device, timeout: Duration, rep: &Reporter) -> Result<String> {
    let label = device.label.clone();

    match device.platform {
        Platform::Simulator => {
            let at = where_of(device);

            rep.try_(format!("booting {label} on {}", at.label()));

            // strictly longer than the deadline above, so the message that comes
            // back is the one that names the device rather than the host
            let backstop = timeout + Duration::from_secs(5);

            tokio::time::timeout(timeout, simctl::boot(&at, simctl::udid(device)?, backstop))
                .await
                .map_err(|_| anyhow!("{label} was still not up after {}s", timeout.as_secs()))??;

            Ok(format!("booted {label}"))
        }

        Platform::Emulator => {
            let at = where_of(device);

            rep.try_(format!("booting {label} on {}", at.label()));

            avd::boot(&at, &label).await?;

            let began = std::time::Instant::now();

            loop {
                if let Some(serial) = avd::booted(&at, &label).await {
                    return Ok(format!("{label} came up as {serial}"));
                }

                if began.elapsed() >= timeout {
                    bail!(
                        "{label} was still not up after {}s; see $TMPDIR/phone-emulator-{label}.log on {}",
                        timeout.as_secs(),
                        at.label()
                    );
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        other => bail!(
            "phone starts and stops emulators and simulators; {label} is {}",
            match other {
                Platform::Ios => "an iPhone",
                _ => "a handset",
            }
        ),
    }
}

pub async fn stop(device: &Device, reach: &Reach) -> Result<String> {
    let label = &device.label;

    if !running(reach) {
        return Ok(format!("{label} is not running"));
    }

    match device.platform {
        Platform::Simulator => {
            simctl::shutdown(&where_of(device), simctl::udid(device)?).await?
        }

        Platform::Emulator => {
            let serial = reach
                .serial()
                .ok_or_else(|| anyhow!("{label} holds no transport to ask to exit"))?;

            avd::shutdown(&where_of(device), serial).await?;
        }

        other => bail!(
            "phone starts and stops emulators and simulators; {label} is {}",
            match other {
                Platform::Ios => "an iPhone",
                _ => "a handset",
            }
        ),
    }

    Ok(format!("stopped {label}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transport column is a name this program invented for a forwarded
    /// server; printing it invites the reader to pass it back to something.
    #[test]
    fn a_forward_is_listed_in_the_shape_it_was_asked_for() {
        let listed = forwards(
            "host-16 tcp:8081 tcp:8081\nhost-16 tcp:3000 tcp:9000",
            "rose",
        );

        assert_eq!(listed, ["8081 -> rose:8081", "3000 -> rose:9000"]);
    }

    #[test]
    fn a_device_with_no_forwards_lists_nothing_rather_than_a_blank_row() {
        assert!(forwards("", "rose").is_empty());
    }

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

    /// The whole point of the thumbnail: mako holds one decoded buffer per
    /// notification in its history, and a phone frame decodes to ~12 MB.
    #[test]
    fn a_preview_is_scaled_down_before_it_reaches_the_daemon() {
        let mut frame = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1206, 2622)
            .write_to(&mut frame, image::ImageFormat::Png)
            .unwrap();

        let thumb = thumbnail(&frame.into_inner()).unwrap();
        let decoded = image::load_from_memory(&thumb).unwrap();

        assert!(
            decoded.width() <= PREVIEW_EDGE && decoded.height() <= PREVIEW_EDGE,
            "a {}x{} icon still costs the daemon a full frame",
            decoded.width(),
            decoded.height()
        );
    }

    #[test]
    fn bytes_that_do_not_decode_leave_the_notification_without_an_icon() {
        assert!(thumbnail(b"not an image").is_none());
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

    /// The bug this closes: a device with no host was "not on any known host",
    /// so a `phone` running on the mac could not drive the simulator sitting
    /// next to it. Absent means here, which is the one machine it cannot mean
    /// anything else.
    #[test]
    fn a_device_that_hangs_off_nothing_is_driven_from_here() {
        let mut device = Device::new("udid", "iPhone 17", Platform::Simulator);

        assert_eq!(where_of(&device), Where::Here);
        assert!(host_of(&device).is_err());

        device.host = Some("rose".to_string());
        assert_eq!(where_of(&device), Where::On("rose".to_string()));

        // a host recorded as empty is no host, or a survey that wrote one would
        // send every command to a machine called ""
        device.host = Some(String::new());
        assert_eq!(where_of(&device), Where::Here);
    }
}
