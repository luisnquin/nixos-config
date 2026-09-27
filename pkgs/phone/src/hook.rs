use std::io::Read;

use clap::ValueEnum;
use serde_json::{json, Value};

const NUDGE: &str = "phone CLI drives these devices (boot, leases, memory budget, taps, screenshots); load the phone skill and prefer it over raw adb/emulator/simctl.";
const PROGRAMS: &[&str] = &["adb", "emulator", "avdmanager", "simctl"];
const PRELUDE: &[&str] = &["nohup", "sudo", "env", "exec", "time", "nice", "command"];

#[derive(Copy, Clone, ValueEnum)]
pub enum Harness {
    Claude,
    Codex,
    Cursor,
}

pub fn run(harness: Harness) {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    if let Some(out) = respond(harness, &input) {
        println!("{out}");
    }
}

fn respond(harness: Harness, input: &str) -> Option<String> {
    let hit = command_of(input).is_some_and(|c| raw(&c));

    match (harness, hit) {
        (Harness::Claude | Harness::Codex, true) => Some(
            json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": NUDGE}})
                .to_string(),
        ),
        (Harness::Cursor, true) => Some(json!({"additional_context": NUDGE}).to_string()),
        (Harness::Cursor, false) => Some("{}".to_string()),
        (_, false) => None,
    }
}

fn command_of(input: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input).ok()?;

    match v.pointer("/tool_input/command").or_else(|| v.get("command"))? {
        Value::String(s) => Some(s.clone()),
        Value::Array(words) => Some(words.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")),
        _ => None,
    }
}

fn raw(command: &str) -> bool {
    command
        .split(['|', ';', '&', '\n', '(', ')', '`', '\'', '"'])
        .any(|segment| leads(segment.split_whitespace()))
}

fn leads<'a>(mut words: impl Iterator<Item = &'a str>) -> bool {
    let Some(first) = words.find(|w| !(w.contains('=') || w.starts_with('-') || PRELUDE.contains(&name(w)))) else {
        return false;
    };

    match name(first) {
        "xcrun" => words.next() == Some("simctl"),
        "ssh" | "sh" | "bash" | "zsh" => words.any(|w| PROGRAMS.contains(&name(w))),
        program => PROGRAMS.contains(&program),
    }
}

fn name(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_device_tooling_is_caught() {
        for line in [
            "ssh rose 'nohup ~/Library/Android/sdk/emulator/emulator -avd pixel_7-api36-b -port 5556 -no-window >/tmp/e.log 2>&1 &'",
            "adb -s emulator-5556 shell getprop sys.boot_completed",
            "xcrun simctl boot ABCD",
            "ANDROID_SERIAL=emulator-5554 adb shell input tap 10 10",
            "cd app && $ANDROID_HOME/platform-tools/adb install -r app.apk",
            "sleep 5; avdmanager list avd",
            "ssh rose adb devices",
            "bash -lc 'adb devices'",
        ] {
            assert!(raw(line), "{line}");
        }
    }

    #[test]
    fn everything_else_stays_silent() {
        for line in [
            "phone device boot pixel_7",
            "grep adb file",
            "rg -n emulator src/",
            "git commit -m \"fix emulator\"",
            "cargo test emulator",
            "cat src/emulator.rs",
            "xcrun --find clang",
            "ssh rose uptime",
            "",
        ] {
            assert!(!raw(line), "{line}");
        }
    }

    #[test]
    fn claude_and_codex_get_context_without_a_decision() {
        let input = r#"{"tool_name":"Bash","tool_input":{"command":"adb devices"}}"#;

        for harness in [Harness::Claude, Harness::Codex] {
            let out: Value = serde_json::from_str(&respond(harness, input).unwrap()).unwrap();

            assert_eq!(out["hookSpecificOutput"]["additionalContext"], NUDGE);
            assert!(out["hookSpecificOutput"].get("permissionDecision").is_none());
        }
    }

    #[test]
    fn misses_and_garbage_stay_silent() {
        for input in ["", "not json", "{}", r#"{"tool_input":{"command":"ls"}}"#, r#"{"tool_input":{"command":42}}"#] {
            assert_eq!(respond(Harness::Claude, input), None);
            assert_eq!(respond(Harness::Cursor, input).as_deref(), Some("{}"));
        }
    }
}
