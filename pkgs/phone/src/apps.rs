//! Starting an app, stopping it, and handing it a URL.
//!
//! Without these the only way into an app is to press its icon, which needs the
//! launcher to be in front — so the first act of driving a screen depends on
//! which screen is already up. These three do not care what is on screen.
//!
//! Each verb is one device-side script rather than a sequence of calls, because
//! every `adb shell` is a round trip to whichever host holds the device, and the
//! useful version of `launch` takes three of them.

use std::time::Duration;

use anyhow::{anyhow, bail, Result};

use crate::adb::{self, Server};
use crate::connect::attached_serial;
use crate::model::{Device, Platform};
use crate::{simctl, ssh};

/// Long enough for a cold start on a loaded emulator, short enough that a
/// launch which is never going to happen is reported rather than waited on.
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);

const STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// A package name on Android, a bundle id on iOS. The two are the same shape,
/// which is why one check covers both.
///
/// Everything here ends up inside a shell command on the device, so this is
/// also what keeps a name from carrying syntax rather than an identifier.
pub fn app_id(app: &str) -> Result<&str> {
    if app.is_empty()
        || !app
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        bail!("invalid app id: {app}");
    }

    Ok(app)
}

/// A URL is not an identifier and cannot be validated down to a charset — a
/// query string is allowed to hold nearly anything. What it must not do is end
/// the quoting it is about to sit inside, so that is the whole of the check.
pub fn url(raw: &str) -> Result<&str> {
    if raw.is_empty() || raw.contains('\'') || raw.chars().any(char::is_whitespace) {
        bail!("a url cannot be empty or hold quotes or spaces: {raw}");
    }

    if !raw.contains(':') {
        bail!("{raw} has no scheme; a url to open looks like https://… or myapp://…");
    }

    Ok(raw)
}

/// The launcher intent first, which is what a user pressing the icon sends. An
/// app whose launchable activity is not declared the usual way answers that
/// with an error and exit 0 alike, so the fallback asks the package manager
/// which component to name and starts it outright.
fn launch_script(app: &str) -> String {
    format!(
        r#"out=$(am start -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p {app} 2>&1)
case "$out" in
  *Error*) ;;
  *) echo "$out"; exit 0 ;;
esac
comp=$(cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER {app} 2>/dev/null | tail -1)
case "$comp" in
  */*) exec am start -n "$comp" 2>&1 ;;
  *) echo "$out"; exit 1 ;;
esac"#
    )
}

/// `args` reach the app as its own `argv`, which is how a runtime that would
/// otherwise sit on a menu can be told what to start. Only a simulator takes
/// them: `am start` sends an intent rather than spawning a process, so Android
/// has no argv to put them in, and a manifest declaring them for it is refused
/// when it is read rather than ignored here.
pub async fn launch(
    server: &Server,
    device: &Device,
    app: &str,
    args: &[String],
) -> Result<String> {
    let app = app_id(app)?;

    if device.platform == Platform::Ios {
        bail!("cannot launch an app on an iPhone from here");
    }

    if device.platform == Platform::Simulator {
        let mut argv = vec![simctl::udid(device)?, app];

        argv.extend(args.iter().map(String::as_str));

        let host = crate::actions::host_of(device)?;
        let ran = ssh::run(
            host,
            // `"$@"` rather than a fixed `"$1" "$2"`, so that however many
            // arguments the manifest declares arrive as that many words
            r#"xcrun simctl launch "$@""#,
            &argv,
            LAUNCH_TIMEOUT,
        )
        .await?;

        if !ran.ok() {
            bail!("{}", missing(&ran.said, app));
        }

        // simctl answers `com.example.app: 51234`
        let said = ran.text();

        return Ok(match said.rsplit_once(": ") {
            Some((_, pid)) => format!("launched {app} on {} (pid {})", device.label, pid.trim()),
            None => format!("launched {app} on {}", device.label),
        });
    }

    if !args.is_empty() {
        bail!("{} takes no launch arguments", device.label);
    }

    let serial = attached(server, device).await?;
    let script = launch_script(app);

    let out = adb::run_timeout(server, &["-s", &serial, "shell", &script], LAUNCH_TIMEOUT).await?;

    let said = format!("{}{}", out.stdout, out.stderr);

    if !out.ok() || said.contains("Error") {
        bail!("{}", missing(&said, app));
    }

    // the pid is the only proof that separates a started app from an intent the
    // system accepted and dropped
    match adb::pidof(server, &serial, app).await {
        Some(pid) => Ok(format!("launched {app} on {} (pid {pid})", device.label)),
        None => Ok(format!(
            "started {app} on {}, but nothing is running under that name yet",
            device.label
        )),
    }
}

/// The three ways a launch fails read very differently on the wire and mean the
/// same thing, so all of them are answered with the step that fixes it. A
/// simulator names no bundle and no reason, only `code=4`.
fn missing(said: &str, app: &str) -> String {
    let said = said.trim();

    if said.contains("does not exist")
        || said.contains("unable to resolve")
        || said.contains("FBSOpenApplicationServiceErrorDomain, code=4")
        || said.is_empty()
    {
        return format!("nothing launchable named {app}; `phone app install` it first");
    }

    detail(said).to_string()
}

pub async fn stop(server: &Server, device: &Device, app: &str) -> Result<String> {
    let app = app_id(app)?;

    if device.platform == Platform::Ios {
        bail!("cannot stop an app on an iPhone from here");
    }

    if device.platform == Platform::Simulator {
        let ran = on_host(
            device,
            r#"xcrun simctl terminate "$1" "$2""#,
            app,
            STOP_TIMEOUT,
        )
        .await?;

        // simctl says `found nothing to terminate` and fails for an app that
        // was not up, which is the state being asked for rather than an error
        if !ran.ok() {
            return match ran.said.contains("found nothing to terminate") {
                true => Ok(format!("{app} was not running on {}", device.label)),
                false => bail!("{}", ran.said),
            };
        }

        return Ok(format!("stopped {app} on {}", device.label));
    }

    let serial = attached(server, device).await?;

    // `am force-stop` succeeds against a package that was never running and one
    // that does not exist alike, so the pid is read first or there is nothing to
    // report but a guess
    let script = format!("pidof {app} 2>/dev/null; am force-stop {app} >/dev/null 2>&1");

    let out = adb::run_timeout(server, &["-s", &serial, "shell", &script], STOP_TIMEOUT).await?;

    if !out.ok() {
        bail!("{}", out.stderr.trim());
    }

    Ok(match out.trimmed().split_whitespace().next() {
        Some(pid) => format!("stopped {app} on {} (was pid {pid})", device.label),
        None => format!("{app} was not running on {}", device.label),
    })
}

pub async fn open(server: &Server, device: &Device, raw: &str) -> Result<String> {
    let raw = url(raw)?;

    if device.platform == Platform::Ios {
        bail!("cannot open a url on an iPhone from here");
    }

    if device.platform == Platform::Simulator {
        let ran = on_host(
            device,
            r#"xcrun simctl openurl "$1" "$2""#,
            raw,
            LAUNCH_TIMEOUT,
        )
        .await?;

        if !ran.ok() {
            bail!("{}", unhandled(&ran.said, raw));
        }

        return Ok(format!("opened {raw} on {}", device.label));
    }

    let serial = attached(server, device).await?;
    let script = format!("am start -a android.intent.action.VIEW -d '{raw}' 2>&1");

    let out = adb::run_timeout(server, &["-s", &serial, "shell", &script], LAUNCH_TIMEOUT).await?;
    let said = format!("{}{}", out.stdout, out.stderr);

    if !out.ok() || said.contains("Error") {
        bail!("{}", unhandled(&said, raw));
    }

    Ok(format!("opened {raw} on {}", device.label))
}

/// Whether `app` is on the device at all.
///
/// The other half of the freshness question `up` asks: a build can be current
/// and the device still not carry it — a wiped emulator, a fresh AVD, an
/// uninstall between runs. Answering it costs one round trip and saves a build
/// that would otherwise be skipped as up to date.
pub async fn installed(server: &Server, device: &Device, app: &str) -> Result<bool> {
    let app = app_id(app)?;

    if device.platform == Platform::Ios {
        bail!("cannot read what is installed on an iPhone from here");
    }

    if device.platform == Platform::Simulator {
        // `get_app_container` exits non-zero for a bundle id the simulator does
        // not carry, which is the whole answer
        let ran = on_host(
            device,
            r#"xcrun simctl get_app_container "$1" "$2""#,
            app,
            LAUNCH_TIMEOUT,
        )
        .await?;

        return Ok(ran.ok());
    }

    let serial = attached(server, device).await?;

    // `pm list packages <name>` matches on substring, so the answer has to be
    // compared rather than counted: `app.example` would report `app.example.dev`
    // as itself
    let script = format!("pm list packages {app}");
    let out = adb::run_timeout(server, &["-s", &serial, "shell", &script], LAUNCH_TIMEOUT).await?;

    if !out.ok() {
        bail!("{}", out.stderr.trim());
    }

    Ok(out
        .stdout
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .any(|name| name == app))
}

/// A url nothing is registered for is the common failure and both platforms
/// describe it in their own words, neither of which names the scheme. On a
/// simulator the words are a number: `LSApplicationWorkspaceErrorDomain` 115.
fn unhandled(said: &str, raw: &str) -> String {
    let said = said.trim();
    let scheme = raw.split_once(':').map(|(s, _)| s).unwrap_or(raw);

    if said.contains("unable to resolve")
        || said.contains("NSOSStatusErrorDomain")
        || said.contains("LSApplicationWorkspaceErrorDomain, code=115")
    {
        return format!("no app on this device handles {scheme}: urls");
    }

    detail(said).to_string()
}

/// The line worth printing out of what a device said. Android puts it first and
/// starts it with `Error`; a simulator buries it under a line naming a domain
/// and a code, and repeats it under a second one. Neither header is readable,
/// and printing all four lines to say one thing is worse than printing none.
fn detail(said: &str) -> &str {
    let lines = || {
        said.lines().map(str::trim).filter(|l| {
            !l.is_empty()
                && !l.starts_with("An error was encountered")
                && !l.starts_with("Underlying error")
        })
    };

    lines()
        .find(|l| l.starts_with("Error"))
        .or_else(|| lines().next())
        .unwrap_or(said)
}

/// One `simctl` verb against a simulator, always in the shape `<verb> <udid>
/// <thing>`. It goes through `ssh::run` rather than a plain command because the
/// ssh session's own exit code cannot be trusted: a brokered one is 0 whatever
/// simctl did, which is the difference between reporting a launch and reporting
/// that an app nobody installed started fine.
async fn on_host(device: &Device, verb: &str, arg: &str, limit: Duration) -> Result<ssh::Ran> {
    let host = crate::actions::host_of(device)?;

    ssh::run(host, verb, &[simctl::udid(device)?, arg], limit).await
}

async fn attached(server: &Server, device: &Device) -> Result<String> {
    attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_an_app_id_that_could_carry_shell_syntax() {
        assert!(app_id("com.example.app").is_ok());
        assert!(app_id("host.exp.Exponent").is_ok());
        assert!(app_id("com.example app; rm -rf /").is_err());
        assert!(app_id("$(id)").is_err());
        assert!(app_id("").is_err());
    }

    /// A url is quoted on the device side rather than validated to a charset,
    /// so the one thing that must not get through is the quote itself.
    #[test]
    fn refuses_a_url_that_could_end_its_own_quoting() {
        assert!(url("https://example.com/a?b=1&c=2").is_ok());
        assert!(url("exp+cuenta-cero://expo-development-client/?url=x").is_ok());
        assert!(url("https://example.com/'; reboot; '").is_err());
        assert!(url("https://example.com/a b").is_err());
    }

    /// Without a scheme `am start -d` resolves nothing and says so in words
    /// that do not mention the url, which is a round trip to learn a typo.
    #[test]
    fn refuses_a_url_with_no_scheme() {
        let err = url("example.com").unwrap_err().to_string();

        assert!(err.contains("no scheme"), "{err}");
    }

    #[test]
    fn a_launch_that_resolves_nothing_says_what_to_do_about_it() {
        let said = missing("Error: Activity class does not exist.", "com.example.app");

        assert!(said.contains("phone app install"), "{said}");
    }

    /// The fallback has to run for an app whose launcher activity is missing
    /// from the intent, and must not run when the first start worked — the
    /// second `am start` would relaunch an app already in front.
    #[test]
    fn the_launcher_intent_is_tried_before_the_package_manager_is_asked() {
        let script = launch_script("com.example.app");
        let (first, rest) = script.split_once("resolve-activity").unwrap();

        assert!(first.contains("category.LAUNCHER -p com.example.app"));
        assert!(first.contains("exit 0"), "a clean start must stop there");
        assert!(rest.contains("am start -n"));
    }

    /// A simulator identifies both failures by a number and says nothing else
    /// about them, so the number is the only thing there is to recognise.
    #[test]
    fn a_simulator_failure_is_read_off_the_code_it_carries() {
        const NOT_INSTALLED: &str = "An error was encountered processing the command (domain=FBSOpenApplicationServiceErrorDomain, code=4):\nSimulator device failed to launch com.example.nope.";
        const NO_HANDLER: &str = "An error was encountered processing the command (domain=LSApplicationWorkspaceErrorDomain, code=115):\nSimulator device failed to open exp+nope://x.";

        assert!(missing(NOT_INSTALLED, "com.example.nope").contains("phone app install"));
        assert_eq!(
            unhandled(NO_HANDLER, "exp+nope://x"),
            "no app on this device handles exp+nope: urls"
        );
    }

    /// Four lines of domains and codes to say one thing, and the sentence in
    /// words is never the first of them.
    #[test]
    fn an_unrecognised_simulator_failure_prints_its_sentence_not_its_domain() {
        let said = "An error was encountered processing the command (domain=X, code=9):\nSimulator device refused.\nUnderlying error (domain=X, code=9):\n\tsomething";

        assert_eq!(detail(said), "Simulator device refused.");
    }

    #[test]
    fn an_unhandled_url_is_reported_by_its_scheme() {
        let said = unhandled(
            "Error: Activity not started, unable to resolve Intent",
            "exp+app://x",
        );

        assert_eq!(said, "no app on this device handles exp+app: urls");
    }
}
