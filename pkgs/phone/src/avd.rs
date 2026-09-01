//! Android virtual devices as they sit on disk, whether or not one is running.
//! `adb` only ever sees a booted emulator, so everything here goes to the SDK
//! instead: what could be started, and starting it.

use std::time::Duration;

use anyhow::{bail, Result};

use crate::ssh::Where;

/// How long an emulator is given to leave adb after being asked to exit. Long
/// enough for one that is writing its snapshot out, short enough that a wedged
/// process is reported rather than waited on.
const EXIT: Duration = Duration::from_secs(60);

/// `emulator` ships inside the SDK rather than in any system path, and a
/// non-interactive ssh reads no login profile, so the login shell is asked for
/// its own PATH the way host probing is, then the usual SDK roots are tried.
/// The SDK roots are a fallback rather than an override: an `emulator` already
/// on PATH may be a wrapper that supplies the libraries the bare SDK binary
/// cannot find on its own, and putting the SDK first would shadow it.
const SDK: &str = r#"PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
command -v emulator >/dev/null 2>&1 || for dir in \
  "$ANDROID_HOME" "$ANDROID_SDK_ROOT" "$HOME/Library/Android/sdk" "$HOME/Android/Sdk"; do
  [ -x "$dir/emulator/emulator" ] && { PATH="$dir/emulator:$PATH"; break; }
done
export PATH"#;

/// Every AVD defined on `at`, booted or not.
pub async fn list(at: &Where) -> Vec<String> {
    let script = format!("{SDK}\nexec emulator -list-avds 2>/dev/null");

    at.text(&script, &[], Duration::from_secs(25))
        .await
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains(' '))
        .map(str::to_string)
        .collect()
}

/// Starts `name` and returns as soon as the process is away. Readiness is a
/// separate question — the window appears long before the system is up — and
/// `booted` is what answers it.
///
/// All three descriptors are redirected because ssh holds the channel open
/// while any of them is, so a plain background job would hang the caller until
/// the emulator exited.
pub async fn boot(at: &Where, name: &str) -> Result<()> {
    let script = format!(
        r#"{SDK}
command -v emulator >/dev/null 2>&1 || {{ echo "no emulator binary in the SDK" >&2; exit 1; }}
log="${{TMPDIR:-/tmp}}/phone-emulator-$1.log"
nohup emulator -avd "$1" -no-boot-anim >"$log" 2>&1 </dev/null &
echo started"#
    );

    let out = at
        .run(&script, &[name])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("{}: {e}", at.label()))?;

    if !String::from_utf8_lossy(&out.stdout).contains("started") {
        let why = String::from_utf8_lossy(&out.stderr);
        let why = why.trim();

        bail!(
            "{}: could not start {name}{}",
            at.label(),
            if why.is_empty() {
                String::new()
            } else {
                format!(": {why}")
            }
        );
    }

    Ok(())
}

/// The serial `name` answers to once it is up, and `None` while it is not. The
/// AVD name is read back off each emulator because the serial is a lease — the
/// port a previous emulator freed goes to the next one to boot.
///
/// `adb` runs on the host rather than through a forward: this is polled, and a
/// tunnel is worth opening once the device is worth surveying.
pub async fn booted(at: &Where, name: &str) -> Option<String> {
    const SCRIPT: &str = r#"PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
for serial in $(adb devices 2>/dev/null | awk '/^emulator-/ {print $1}'); do
  avd=$(adb -s "$serial" shell getprop ro.boot.qemu.avd_name 2>/dev/null | tr -d '\r')
  [ -z "$avd" ] && avd=$(adb -s "$serial" shell getprop ro.kernel.qemu.avd_name 2>/dev/null | tr -d '\r')
  [ "$avd" = "$1" ] || continue
  [ "$(adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ] && echo "$serial"
  break
done"#;

    let out = at.text(SCRIPT, &[name], Duration::from_secs(25)).await;
    let serial = out.lines().next_back().unwrap_or_default().trim();

    (!serial.is_empty()).then(|| serial.to_string())
}

/// Asks the emulator's own console to exit, which is a clean shutdown rather
/// than the kill an unnamed process would get.
pub async fn shutdown(at: &Where, serial: &str) -> Result<()> {
    const SCRIPT: &str = r#"PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
exec adb -s "$1" emu kill"#;

    let out = at
        .run(SCRIPT, &[serial])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("{}: {e}", at.label()))?;

    if !out.status.success() {
        bail!(
            "{}: {}",
            at.label(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    gone(at, serial).await
}

/// Blocks until `serial` has left the host's adb, which is a later moment than
/// the one `emu kill` returns at: the console accepts the word and the process
/// takes seconds more to go, holding the AVD's lock for all of them. Returning
/// before that makes `down` a lie to whatever runs next — the AVD cannot be
/// started again while the instance that owns it is still exiting.
async fn gone(at: &Where, serial: &str) -> Result<()> {
    const LISTED: &str = r#"PATH="$($SHELL -l -c 'printf %s "$PATH"' 2>/dev/null):$PATH"
adb devices 2>/dev/null | awk -v want="$1" '$1 == want { print "listed" }'"#;

    let began = std::time::Instant::now();

    while began.elapsed() < EXIT {
        if !at
            .text(LISTED, &[serial], Duration::from_secs(25))
            .await
            .contains("listed")
        {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    bail!(
        "{serial} was still listed {}s after being asked to exit",
        EXIT.as_secs()
    )
}
