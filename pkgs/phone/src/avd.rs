//! Android virtual devices as they sit on disk, whether or not one is running.
//! `adb` only ever sees a booted emulator, so everything here goes to the SDK
//! instead: what could be started, and starting it.

use std::time::Duration;

use anyhow::{bail, Result};

use crate::ssh::{landed, Where};

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
const SDK: &str = r#"login=$($SHELL -l -c 'printf "%s\n%s" "$ANDROID_AVD_HOME" "$PATH"' 2>/dev/null)
PATH="$(printf '%s\n' "$login" | tail -n 1):$PATH"
avd_home=$(printf '%s\n' "$login" | tail -n 2 | head -n 1)
[ -n "$avd_home" ] || avd_home="${ANDROID_AVD_HOME:-${ANDROID_USER_HOME:-$HOME/.android}/avd}"
command -v emulator >/dev/null 2>&1 || for dir in \
  "$ANDROID_HOME" "$ANDROID_SDK_ROOT" "$HOME/Library/Android/sdk" "$HOME/Android/Sdk"; do
  [ -x "$dir/emulator/emulator" ] && { PATH="$dir/emulator:$PATH"; break; }
done
export PATH"#;

const LIST: &str = r#"read_key() { sed -n "s/^$1 *= *//p" "$dir/config.ini" 2>/dev/null | head -n 1; }
emulator -list-avds 2>/dev/null | while IFS= read -r avd; do
  dir=$(sed -n 's/^path=//p' "$avd_home/$avd.ini" 2>/dev/null)
  [ -d "$dir" ] || dir="$avd_home/$avd.avd"
  printf '%s\t%s\t%s\n' "$avd" "$(read_key 'hw\.device\.name')" "$(read_key 'image\.sysdir\.1')"
done"#;

/// Every AVD defined on `at`, booted or not.
pub async fn list(at: &Where) -> Vec<(String, String)> {
    let script = format!("{SDK}\n{LIST}");

    parse_list(&at.text(&script, &[], Duration::from_secs(25)).await)
}

fn parse_list(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t').map(str::trim);
            let name = fields
                .next()
                .filter(|n| !n.is_empty() && !n.contains(' '))?;
            let device = fields.next().unwrap_or_default();
            let api = fields
                .next()
                .unwrap_or_default()
                .split(['/', ';'])
                .find_map(|part| part.strip_prefix("android-"))
                .map(|n| format!("API {n}"));

            let model = [Some(device.to_string()), api]
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");

            Some((name.to_string(), model))
        })
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

const LOCATE: &str = r#"ini="$avd_home/$1.ini"
[ -f "$ini" ] || { echo "no AVD named $1 in $avd_home" >&2; exit 1; }
dir=$(sed -n 's/^path=//p' "$ini" | head -n 1)
[ -d "$dir" ] || dir="$avd_home/$1.avd"
[ -f "$dir/config.ini" ] || { echo "$dir has no config.ini" >&2; exit 1; }
uname -s
printf '%s\n%s\n' "$avd_home" "$dir"
awk 1 "$ini"
printf '%s\n' "$2"
cat "$dir/config.ini""#;

const COPY: &str = r#"src=$1 dst=$2 ini=$3 ini_text=$4 config=$5
shift 5
if [ -e "$dst" ] || [ -e "$ini" ]; then
  echo "$(basename "$ini" .ini) already exists" >&2
  exit 1
fi
fail() { rm -rf "$dst" "$ini"; exit 1; }
"$@" "$src" "$dst" || fail
rm -rf "$dst/snapshots" "$dst/hardware-qemu.ini" || fail
printf '%s' "$config" >"$dst/config.ini" || fail
printf '%s' "$ini_text" >"$ini" || fail"#;

const SPLIT: &str = "--phone-config--";

#[derive(Debug, PartialEq)]
struct Source {
    os: String,
    home: String,
    dir: String,
    ini: String,
    config: String,
}

#[derive(Debug, PartialEq)]
struct Copy {
    dir: String,
    ini_path: String,
    ini: String,
    config: String,
}

pub async fn clone(at: &Where, name: &str, new: &str) -> Result<()> {
    valid(new)?;

    let script = format!("{SDK}\n{LOCATE}");
    let found = at
        .exec(&script, &[name, SPLIT], Duration::from_secs(25))
        .await?;
    let text = found.text();

    landed(at, "reading the AVD", found)?;

    let source = located(&text)
        .ok_or_else(|| anyhow::anyhow!("{}: could not read {name}'s files", at.label()))?;
    let copy = plan(&source, new);

    let mut args = vec![
        source.dir.as_str(),
        &copy.dir,
        &copy.ini_path,
        &copy.ini,
        &copy.config,
    ];
    args.extend(copy_command(&source.os));

    landed(
        at,
        "copying the AVD",
        at.exec(COPY, &args, Duration::from_secs(600)).await?,
    )
}

fn valid(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && !name.starts_with(['.', '-'])
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));

    if !ok {
        bail!("an AVD name takes only letters, digits, '.', '_' and '-': {name}");
    }

    Ok(())
}

/// The system `cp` by path on macOS: the login PATH can put GNU coreutils
/// first, where `-c` means something else entirely.
fn copy_command(os: &str) -> &'static [&'static str] {
    match os {
        "Darwin" => &["/bin/cp", "-c", "-R"],
        _ => &["cp", "-R", "--reflink=auto"],
    }
}

fn located(text: &str) -> Option<Source> {
    let mut lines = text.splitn(4, '\n');
    let os = lines.next()?.trim().to_string();
    let home = lines.next()?.trim_end_matches('/').to_string();
    let dir = lines.next()?.trim_end_matches('/').to_string();
    let (ini, config) = lines.next()?.split_once(&format!("\n{SPLIT}\n"))?;

    (!home.is_empty() && !dir.is_empty()).then(|| Source {
        os,
        home,
        dir,
        ini: format!("{ini}\n"),
        config: format!("{}\n", config.trim_end_matches('\n')),
    })
}

fn plan(source: &Source, new: &str) -> Copy {
    let beside = |path: &str| match path.trim_end_matches('/').rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{new}.avd"),
        None => format!("{new}.avd"),
    };
    let dir = beside(&source.dir);

    let ini = rewrite(&source.ini, |key, value| match key {
        "path" => Some(dir.clone()),
        "path.rel" => Some(beside(value)),
        _ => None,
    });

    let mut config = rewrite(&source.config, |key, value| match key {
        "AvdId" | "avd.ini.displayname" => Some(new.to_string()),
        _ => value
            .strip_prefix(&source.dir)
            .filter(|rest| rest.is_empty() || rest.starts_with('/'))
            .map(|rest| format!("{dir}{rest}")),
    });

    if !source.config.lines().any(|l| key_of(l) == Some("AvdId")) {
        config.push_str(&format!("AvdId={new}\n"));
    }

    Copy {
        ini_path: format!("{}/{new}.ini", source.home),
        dir,
        ini,
        config,
    }
}

fn key_of(line: &str) -> Option<&str> {
    line.split_once('=').map(|(key, _)| key.trim())
}

fn rewrite(text: &str, change: impl Fn(&str, &str) -> Option<String>) -> String {
    let mut out = String::new();

    for line in text.lines() {
        let swapped = line.split_once('=').and_then(|(key, value)| {
            let pad = &value[..value.len() - value.trim_start().len()];

            change(key.trim(), value.trim()).map(|v| format!("{key}={pad}{v}"))
        });

        out.push_str(swapped.as_deref().unwrap_or(line));
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source {
            os: "Darwin".into(),
            home: "/Users/me/android-staged/avd".into(),
            dir: "/Users/me/android-staged/avd/pixel_7.avd".into(),
            ini: "avd.ini.encoding=UTF-8\n\
                  path=/Users/me/android-staged/avd/pixel_7.avd\n\
                  path.rel=avd/pixel_7.avd\n\
                  target=android-36\n"
                .into(),
            config: "AvdId = pixel_7\n\
                     avd.ini.displayname=Pixel 7\n\
                     hw.ramSize = 2048\n\
                     hw.sdCard.path=/Users/me/android-staged/avd/pixel_7.avd/sdcard.img\n\
                     image.sysdir.1=system-images/android-36/google_apis/arm64-v8a/\n"
                .into(),
        }
    }

    #[test]
    fn reads_back_what_locating_an_avd_printed() {
        let s = source();
        let text = format!(
            "Darwin\n{}/\n{}\n{}{SPLIT}\n{}",
            s.home,
            s.dir,
            s.ini,
            s.config.trim_end()
        );

        assert_eq!(located(&text), Some(s));
        assert_eq!(located("Darwin\n/avd\n"), None);
    }

    #[test]
    fn points_the_copy_at_its_own_directory() {
        let copy = plan(&source(), "zz-copy");

        assert_eq!(copy.dir, "/Users/me/android-staged/avd/zz-copy.avd");
        assert_eq!(copy.ini_path, "/Users/me/android-staged/avd/zz-copy.ini");
        assert_eq!(
            copy.ini,
            "avd.ini.encoding=UTF-8\n\
             path=/Users/me/android-staged/avd/zz-copy.avd\n\
             path.rel=avd/zz-copy.avd\n\
             target=android-36\n"
        );
        assert_eq!(
            copy.config,
            "AvdId = zz-copy\n\
             avd.ini.displayname=zz-copy\n\
             hw.ramSize = 2048\n\
             hw.sdCard.path=/Users/me/android-staged/avd/zz-copy.avd/sdcard.img\n\
             image.sysdir.1=system-images/android-36/google_apis/arm64-v8a/\n"
        );
    }

    #[test]
    fn names_a_copy_whose_config_never_did() {
        let mut s = source();
        s.config = "hw.ramSize=2048\n".into();

        assert_eq!(plan(&s, "b").config, "hw.ramSize=2048\nAvdId=b\n");
    }

    #[test]
    fn a_sibling_directory_is_not_mistaken_for_the_source() {
        let mut s = source();
        s.config = "x=/Users/me/android-staged/avd/pixel_7.avd2/a\n".into();

        assert_eq!(plan(&s, "b").config, format!("{}AvdId=b\n", s.config));
    }

    #[test]
    fn clones_where_the_filesystem_can_and_copies_where_it_cannot() {
        assert_eq!(copy_command("Darwin"), ["/bin/cp", "-c", "-R"]);
        assert_eq!(copy_command("Linux"), ["cp", "-R", "--reflink=auto"]);
    }

    #[test]
    fn refuses_a_name_the_emulator_would() {
        for (name, ok) in [("zz-clone-test_1.2", true), ("Pixel 7", false), ("-avd", false), ("$(x)", false), ("", false)] {
            assert_eq!(valid(name).is_ok(), ok, "{name}");
        }
    }

    fn copied(dir: &std::path::Path, copy: &Copy, src: &std::path::Path) -> std::process::Output {
        let mut args = vec![
            src.to_str().unwrap(),
            &copy.dir,
            &copy.ini_path,
            &copy.ini,
            &copy.config,
        ];
        args.extend(copy_command(if cfg!(target_os = "macos") {
            "Darwin"
        } else {
            "Linux"
        }));

        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(COPY)
            .arg("sh")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
    }

    #[test]
    fn copies_an_avd_without_its_snapshots_and_refuses_to_overwrite() {
        let home = std::env::temp_dir().join(format!("phone-avd-{}", std::process::id()));
        let src = home.join("a.avd");

        std::fs::create_dir_all(src.join("snapshots/default_boot")).unwrap();
        std::fs::write(src.join("config.ini"), "AvdId=a\n").unwrap();
        std::fs::write(src.join("userdata-qemu.img"), "data").unwrap();
        std::fs::write(src.join("hardware-qemu.ini"), "a.avd").unwrap();

        let source = Source {
            os: "Linux".into(),
            home: home.display().to_string(),
            dir: src.display().to_string(),
            ini: format!("path={}\n", src.display()),
            config: "AvdId=a\n".into(),
        };
        let copy = plan(&source, "b");

        assert!(copied(&home, &copy, &src).status.success());

        let dst = home.join("b.avd");

        assert_eq!(
            std::fs::read_to_string(dst.join("config.ini")).unwrap(),
            "AvdId=b\n"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("userdata-qemu.img")).unwrap(),
            "data"
        );
        assert!(!dst.join("snapshots").exists());
        assert!(!dst.join("hardware-qemu.ini").exists());
        assert!(src.join("snapshots/default_boot").exists());
        assert_eq!(
            std::fs::read_to_string(home.join("b.ini")).unwrap(),
            format!("path={}\n", dst.display())
        );

        let again = copied(&home, &copy, &src);

        assert!(!again.status.success());
        assert!(String::from_utf8_lossy(&again.stderr).contains("b already exists"));
        assert!(dst.join("config.ini").exists());

        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn a_failed_copy_leaves_nothing_behind() {
        let home = std::env::temp_dir().join(format!("phone-avd-fail-{}", std::process::id()));

        std::fs::create_dir_all(&home).unwrap();

        let source = Source {
            os: "Linux".into(),
            home: home.display().to_string(),
            dir: home.join("missing.avd").display().to_string(),
            ini: String::new(),
            config: String::new(),
        };
        let copy = plan(&source, "b");

        assert!(!copied(&home, &copy, &home.join("missing.avd"))
            .status
            .success());
        assert!(!home.join("b.avd").exists());
        assert!(!home.join("b.ini").exists());

        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn names_what_an_avd_emulates_from_its_config() {
        let text =
            "pixel_7-api36\tpixel_7\tsystem-images/android-36/google_apis_playstore/arm64-v8a/\n\
                    tablet\t\t\n\
                    INFO    | Storing crashdata in: /tmp\n\
                    legacy\tpixel_4\tsystem-images;android-30;default;x86_64\n";

        assert_eq!(
            parse_list(text),
            vec![
                ("pixel_7-api36".to_string(), "pixel_7 API 36".to_string()),
                ("tablet".to_string(), String::new()),
                ("legacy".to_string(), "pixel_4 API 30".to_string()),
            ]
        );
    }
}
