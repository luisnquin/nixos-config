use std::io::Read;

use clap::ValueEnum;
use serde_json::{json, Map, Value};

const NUDGE: &str = "phone CLI drives these devices (boot, leases, memory budget, taps, screenshots); load the phone skill and prefer it over raw adb/emulator/simctl.";
const PROGRAMS: &[&str] = &["adb", "emulator", "avdmanager", "simctl"];
const READ_ONLY: &[&str] = &["devices", "version", "--version"];
const PRELUDE: &[&str] = &["nohup", "sudo", "env", "exec", "time", "nice", "command"];
const SEPARATORS: &[char] = &['|', ';', '&', '\n', '(', ')', '`', '\'', '"'];

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
    let event: Value = serde_json::from_str(input).unwrap_or(Value::Null);
    let command = command_of(&event);
    let hit = command.as_deref().and_then(nudge);

    match harness {
        Harness::Cursor => Some(match hit {
            Some(nudge) => json!({"additional_context": nudge}).to_string(),
            None => "{}".to_string(),
        }),
        Harness::Codex => hit.map(|nudge| specific(Map::new(), Some(nudge)).to_string()),
        Harness::Claude => claude(&event, command.as_deref(), hit),
    }
}

fn claude(event: &Value, command: Option<&str>, hit: Option<String>) -> Option<String> {
    let mut out = Map::new();

    if let Some(updated) = command.and_then(|c| inject(event, c)) {
        out.insert("updatedInput".into(), updated);
    }

    match out.is_empty() && hit.is_none() {
        true => None,
        false => Some(specific(out, hit).to_string()),
    }
}

fn specific(mut out: Map<String, Value>, hit: Option<String>) -> Value {
    out.insert("hookEventName".into(), "PreToolUse".into());

    if let Some(nudge) = hit {
        out.insert("additionalContext".into(), nudge.into());
    }

    json!({ "hookSpecificOutput": out })
}

fn inject(event: &Value, command: &str) -> Option<Value> {
    if !invokes_phone(command) {
        return None;
    }

    let agent = agent_of(event)?;
    let mut input = event.get("tool_input")?.as_object()?.clone();
    let command = format!("export PHONE_AGENT={agent}; {}", scrub(command));

    input.insert("command".into(), command.into());

    Some(Value::Object(input))
}

fn agent_of(event: &Value) -> Option<String> {
    let token = |key: &str| {
        event
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    };

    let session = token("session_id")?;

    Some(match token("agent_id") {
        Some(agent) => format!("{session}/{agent}"),
        None => session.to_string(),
    })
}

fn scrub(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut rest = command;

    while let Some(at) = rest.find("PHONE_AGENT=") {
        let (before, after) = rest.split_at(at);
        let value = value_len(&after["PHONE_AGENT=".len()..]);

        match before.strip_suffix("export ") {
            Some(head) => {
                out.push_str(head);
                out.push(':');
            }
            None => out.push_str(before),
        }

        rest = &after["PHONE_AGENT=".len() + value..];
    }

    out.push_str(rest);

    out
}

fn value_len(s: &str) -> usize {
    let quoted = |q: char| s[1..].find(q).map_or(s.len(), |end| end + 2);

    match s.chars().next() {
        Some(q @ ('\'' | '"')) => quoted(q),
        _ => s
            .find(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | ')'))
            .unwrap_or(s.len()),
    }
}

fn command_of(event: &Value) -> Option<String> {
    match event.pointer("/tool_input/command").or_else(|| event.get("command"))? {
        Value::String(s) => Some(s.clone()),
        Value::Array(words) => Some(words.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")),
        _ => None,
    }
}

fn invokes_phone(command: &str) -> bool {
    command
        .split(SEPARATORS)
        .any(|segment| lead(&mut segment.split_whitespace()).is_some_and(|w| name(w) == "phone"))
}

// The lease ledger lives on the device host, so checking it here would cost an
// ssh round-trip on every Bash call; the nudge fires whether or not a lease is held.
fn nudge(command: &str) -> Option<String> {
    if phone_runs(command) {
        return None;
    }

    let hits = command.split(SEPARATORS).filter_map(|segment| match steers(segment) {
        true => Some(None),
        false => invocation(segment).filter(|words| !read_only(words)).map(Some),
    });

    hits.reduce(|first, next| first.or(next)).map(|hit| match hit {
        Some(words) if name(words[0]) == "adb" => format!(
            "bare adb skips phone's leases and device targeting; run `phone run -- adb {}` instead.",
            words[1..].join(" ")
        ),
        _ => NUDGE.to_string(),
    })
}

fn invocation(segment: &str) -> Option<Vec<&str>> {
    let mut words = segment.split_whitespace();
    let first = lead(&mut words)?;
    let rest: Vec<&str> = words.collect();
    let whole = || [vec![first], rest.clone()].concat();

    match name(first) {
        "xcrun" => (rest.iter().find(|w| !w.starts_with('-')) == Some(&"simctl")).then(whole),
        "ssh" | "sh" | "bash" | "zsh" => rest.iter().position(|w| PROGRAMS.contains(&name(w))).map(|at| rest[at..].to_vec()),
        program => PROGRAMS.contains(&program).then(whole),
    }
}

fn read_only(words: &[&str]) -> bool {
    name(words[0]) == "adb" && words.get(1).is_some_and(|w| READ_ONLY.contains(w))
}

fn phone_runs(command: &str) -> bool {
    command.split(SEPARATORS).any(|segment| {
        let mut words = segment.split_whitespace();

        lead(&mut words).is_some_and(|w| name(w) == "phone") && words.next() == Some("run")
    })
}

fn steers(segment: &str) -> bool {
    segment
        .split_whitespace()
        .take_while(|w| w.contains('=') || *w == "export" || PRELUDE.contains(&name(w)))
        .any(|w| w.starts_with("ADB_SERVER_SOCKET="))
}

fn lead<'a>(words: &mut impl Iterator<Item = &'a str>) -> Option<&'a str> {
    words.find(|w| !(w.contains('=') || w.starts_with('-') || PRELUDE.contains(&name(w))))
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
            "ssh rose adb reboot",
            "bash -lc 'adb shell ls'",
            "ssh -o BatchMode=yes rose \"adb -s emulator-5554 shell input tap 1 1\"",
            "ADB_SERVER_SOCKET=tcp:rose:5037 ./gradlew connectedAndroidTest",
            "export ADB_SERVER_SOCKET=tcp:rose:5037",
            "env ADB_SERVER_SOCKET=tcp:rose:5037 npx expo run:android",
            "xcrun -v simctl list devices",
            "~/Library/Android/sdk/emulator/emulator -avd pixel_7-api36 -no-window",
        ] {
            assert!(nudge(line).is_some(), "{line}");
        }
    }

    #[test]
    fn everything_else_stays_silent() {
        for line in [
            "phone device boot pixel_7",
            "phone run -- adb shell getprop ro.product.model",
            "phone run -t pixel_7 -- sh -c 'adb install a.apk; adb shell am start x'",
            "grep adb file",
            "rg -n emulator src/",
            "git commit -m \"fix emulator\"",
            "cargo test emulator",
            "cat src/emulator.rs",
            "xcrun --find clang",
            "ssh rose uptime",
            "grep -rn ADB_SERVER_SOCKET= src",
            "echo $ADB_SERVER_SOCKET",
            "",
        ] {
            assert!(nudge(line).is_none(), "{line}");
        }
    }

    #[test]
    fn claude_and_codex_get_context_without_a_decision() {
        let input = r#"{"tool_name":"Bash","tool_input":{"command":"xcrun simctl list"}}"#;

        for harness in [Harness::Claude, Harness::Codex] {
            let out: Value = serde_json::from_str(&respond(harness, input).unwrap()).unwrap();

            assert_eq!(out["hookSpecificOutput"]["additionalContext"], NUDGE);
            assert!(out["hookSpecificOutput"].get("permissionDecision").is_none());
            assert!(out["hookSpecificOutput"].get("updatedInput").is_none());
        }
    }

    fn context(harness: Harness, command: &str) -> Option<String> {
        let input = json!({"tool_name": "Bash", "tool_input": {"command": command}});
        let out: Value = serde_json::from_str(&respond(harness, &input.to_string())?).ok()?;

        assert!(out["hookSpecificOutput"].get("permissionDecision").is_none());

        out["hookSpecificOutput"]["additionalContext"].as_str().map(String::from)
    }

    #[test]
    fn bare_adb_is_pointed_at_phone_run_with_the_same_args() {
        for (command, route) in [
            ("adb -s emulator-5554 shell input tap 1 1", "phone run -- adb -s emulator-5554 shell input tap 1 1"),
            ("ssh rose adb shell pm clear com.x", "phone run -- adb shell pm clear com.x"),
            ("ssh rose 'adb install -r a.apk'", "phone run -- adb install -r a.apk"),
            ("adb devices && $ANDROID_HOME/platform-tools/adb uninstall com.x", "phone run -- adb uninstall com.x"),
        ] {
            for harness in [Harness::Claude, Harness::Codex] {
                let said = context(harness, command).unwrap_or_default();

                assert!(said.contains(&format!("`{route}`")), "{command}: {said}");
                assert!(!said.contains('\n'), "{said}");
            }
        }
    }

    #[test]
    fn read_only_adb_stays_silent() {
        for command in ["adb devices", "adb devices -l", "adb version", "ssh rose adb devices", "bash -lc 'adb devices'"] {
            assert_eq!(context(Harness::Claude, command), None, "{command}");
        }
    }

    #[test]
    fn misses_and_garbage_stay_silent() {
        for input in ["", "not json", "{}", r#"{"tool_input":{"command":"ls"}}"#, r#"{"tool_input":{"command":42}}"#] {
            assert_eq!(respond(Harness::Claude, input), None);
            assert_eq!(respond(Harness::Cursor, input).as_deref(), Some("{}"));
        }
    }

    fn updated(input: Value) -> Value {
        let out: Value = serde_json::from_str(&respond(Harness::Claude, &input.to_string()).unwrap()).unwrap();

        assert_eq!(out["hookSpecificOutput"]["hookEventName"], "PreToolUse");

        out["hookSpecificOutput"]["updatedInput"].clone()
    }

    #[test]
    fn a_subagent_running_phone_gets_its_own_identity() {
        let input = json!({
            "session_id": "1f636eb3-8a91",
            "agent_id": "a6a8204d",
            "tool_name": "Bash",
            "tool_input": {"command": "phone tap Login", "description": "tap", "timeout": 5000},
        });
        let out = updated(input);

        assert_eq!(out["command"], "export PHONE_AGENT=1f636eb3-8a91/a6a8204d; phone tap Login");
        assert_eq!(out["description"], "tap");
        assert_eq!(out["timeout"], 5000);
    }

    #[test]
    fn the_main_thread_is_its_session() {
        let input = json!({"session_id": "s1", "tool_input": {"command": "cd x && phone up"}});

        assert_eq!(updated(input)["command"], "export PHONE_AGENT=s1; cd x && phone up");
    }

    #[test]
    fn a_value_already_in_the_command_is_overwritten() {
        for (command, scrubbed) in [
            ("PHONE_AGENT=other phone tap x", " phone tap x"),
            ("export PHONE_AGENT='a b'; phone up", ":; phone up"),
            ("env PHONE_AGENT=\"q\" phone up", "env  phone up"),
        ] {
            let input = json!({"session_id": "s1", "tool_input": {"command": command}});

            assert_eq!(updated(input)["command"], format!("export PHONE_AGENT=s1; {scrubbed}"));
        }
    }

    #[test]
    fn commands_without_phone_are_left_alone() {
        for command in ["ls", "cat phone.toml", "grep phone src", "echo phone"] {
            let input = json!({"session_id": "s1", "tool_input": {"command": command}});

            assert_eq!(respond(Harness::Claude, &input.to_string()), None, "{command}");
        }
    }

    #[test]
    fn hostile_ids_and_codex_are_never_rewritten() {
        for (harness, session) in [(Harness::Claude, "x; rm -rf ~"), (Harness::Codex, "s1")] {
            let input = json!({"session_id": session, "tool_input": {"command": "phone up"}});

            assert_eq!(respond(harness, &input.to_string()), None);
        }
    }
}
