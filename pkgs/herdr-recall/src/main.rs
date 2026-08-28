//! Records what each herdr pane was running so a restart does not lose it.
//!
//! Agent panes are swept from the socket API, which carries the session id herdr
//! itself resumes from. Shell panes have no such event, so `shell/hook.zsh`
//! reports each command as it starts and finishes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use tinyjson::JsonValue;

const METADATA_SOURCE: &str = "herdr-recall";
const TOKEN: &str = "last";
const TOKEN_MAX_LEN: usize = 28;
const COMMAND_MAX_LEN: usize = 512;
/// Entries for panes herdr no longer knows about are kept this long, then swept.
const STALE_SECS: u64 = 60 * 60 * 24 * 30;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("preexec") => preexec(args.get(1).map(String::as_str).unwrap_or_default()),
        Some("precmd") => precmd(
            args.get(1).and_then(|code| code.parse::<i64>().ok()),
            args.get(2).map(String::as_str),
        ),
        Some("show") => show(args.iter().any(|arg| arg == "--json")),
        _ => sync(),
    }
}

// ---- entries ----

#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Entry {
    pane_id: String,
    workspace: Option<String>,
    tab: Option<String>,
    cwd: Option<String>,
    /// Last command the shell hook saw, whether or not it is still running.
    command: Option<String>,
    running: bool,
    exit: Option<i64>,
    agent: Option<String>,
    /// `source`, `kind` and `value` of the agent session herdr would resume.
    session: Option<(String, String, String)>,
    updated: u64,
}

impl Entry {
    fn parse(raw: &str) -> Option<Self> {
        let json: JsonValue = raw.parse().ok()?;
        let session = match (
            string(&json, "session_source"),
            string(&json, "session_kind"),
            string(&json, "session_value"),
        ) {
            (Some(source), Some(kind), Some(value)) => Some((source, kind, value)),
            _ => None,
        };
        Some(Self {
            pane_id: string(&json, "pane_id")?,
            workspace: string(&json, "workspace"),
            tab: string(&json, "tab"),
            cwd: string(&json, "cwd"),
            command: string(&json, "command"),
            running: boolean(&json, "running"),
            exit: number(&json, "exit").map(|exit| exit as i64),
            agent: string(&json, "agent"),
            session,
            updated: number(&json, "updated").unwrap_or_default() as u64,
        })
    }

    fn to_json(&self) -> JsonValue {
        let mut object: HashMap<String, JsonValue> = HashMap::new();
        let mut put = |key: &str, value: Option<String>| {
            if let Some(value) = value {
                object.insert(key.to_string(), JsonValue::String(value));
            }
        };
        put("pane_id", Some(self.pane_id.clone()));
        put("workspace", self.workspace.clone());
        put("tab", self.tab.clone());
        put("cwd", self.cwd.clone());
        put("command", self.command.clone());
        put("agent", self.agent.clone());
        if let Some((source, kind, value)) = self.session.clone() {
            put("session_source", Some(source));
            put("session_kind", Some(kind));
            put("session_value", Some(value));
        }
        object.insert("running".to_string(), JsonValue::Boolean(self.running));
        if let Some(exit) = self.exit {
            object.insert("exit".to_string(), JsonValue::Number(exit as f64));
        }
        object.insert(
            "updated".to_string(),
            JsonValue::Number(self.updated as f64),
        );
        JsonValue::Object(object)
    }

    /// What to type to get this pane back. Agent panes win: a pane that ran a
    /// shell and then launched an agent should come back as the agent.
    ///
    /// An agent whose session was never reported still gets a line — knowing the
    /// pane held claude is worth more than dropping it for want of an id.
    fn restore(&self) -> Option<String> {
        self.resume()
            .or_else(|| self.command.clone())
            .or_else(|| self.agent.clone())
    }

    fn resume(&self) -> Option<String> {
        let (source, kind, value) = self.session.as_ref()?;
        let agent = source.strip_prefix("herdr:")?;
        let argv = match (agent, kind.as_str()) {
            ("claude", "id") => format!("claude --resume {value}"),
            ("codex", "id") => format!("codex resume {value}"),
            ("copilot", "id") => format!("copilot --resume={value}"),
            ("droid" | "devin" | "hermes" | "qwen", "id") => format!("{agent} --resume {value}"),
            ("opencode", "id") => format!("opencode --session {value}"),
            ("pi", _) => format!("pi --session {value}"),
            ("omp", _) => format!("omp --resume={value}"),
            _ => return None,
        };
        Some(argv)
    }
}

// ---- store: one file per pane, so concurrent writers never clobber each other ----

fn store_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state")
        .join("herdr-recall")
        .join("panes")
}

/// Pane ids are shaped like `w1:p2`; percent-encode anything awkward in a name.
fn key(pane_id: &str) -> String {
    let mut key = String::with_capacity(pane_id.len());
    for byte in pane_id.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => key.push(byte as char),
            _ => key.push_str(&format!("%{byte:02X}")),
        }
    }
    key
}

fn load(pane_id: &str) -> Option<Entry> {
    let raw = std::fs::read_to_string(store_dir().join(format!("{}.json", key(pane_id)))).ok()?;
    Entry::parse(&raw)
}

fn save(entry: &Entry) {
    let dir = store_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(raw) = entry.to_json().stringify() else {
        return;
    };
    let path = dir.join(format!("{}.json", key(&entry.pane_id)));
    let temp = path.with_extension("json.tmp");
    if std::fs::write(&temp, raw).is_ok() {
        let _ = std::fs::rename(&temp, &path);
    }
}

fn load_all() -> Vec<Entry> {
    let Ok(dir) = std::fs::read_dir(store_dir()) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = dir
        .filter_map(Result::ok)
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|file| std::fs::read_to_string(file.path()).ok())
        .filter_map(|raw| Entry::parse(&raw))
        .collect();
    entries.sort_by(|a, b| a.pane_id.cmp(&b.pane_id));
    entries
}

// ---- commands ----

fn preexec(cmdline: &str) {
    let Some(pane_id) = pane_id() else { return };
    let Some(command) = clean(cmdline) else { return };
    let mut entry = load(&pane_id).unwrap_or_default();
    entry.pane_id = pane_id;
    entry.command = Some(command);
    entry.running = true;
    entry.exit = None;
    entry.updated = now();
    save(&entry);
}

fn precmd(exit: Option<i64>, cwd: Option<&str>) {
    let Some(pane_id) = pane_id() else { return };
    let Some(mut entry) = load(&pane_id) else { return };
    if !entry.running {
        return;
    }
    entry.running = false;
    entry.exit = exit;
    if let Some(cwd) = cwd {
        entry.cwd = Some(cwd.to_string());
    }
    entry.updated = now();
    save(&entry);
    report(&entry);
}

/// Refresh every live pane from the socket API, then drop long-dead entries.
fn sync() {
    let panes = herdr(&["pane", "list"])
        .as_ref()
        .and_then(|json| array(json, &["result", "panes"]).cloned())
        .unwrap_or_default();

    let mut live: Vec<String> = Vec::new();
    for pane in &panes {
        let Some(pane_id) = string(pane, "pane_id") else {
            continue;
        };
        live.push(pane_id.clone());
        let mut entry = load(&pane_id).unwrap_or_default();
        entry.pane_id = pane_id;
        entry.workspace = string(pane, "workspace_id");
        entry.tab = string(pane, "tab_id");
        entry.cwd = string(pane, "cwd").or(entry.cwd);
        entry.agent = string(pane, "agent");
        entry.session = at(pane, &["agent_session"]).and_then(|session| {
            Some((
                string(session, "source")?,
                string(session, "kind")?,
                string(session, "value")?,
            ))
        });
        entry.updated = now();
        save(&entry);
    }

    prune(&live);
}

fn prune(live: &[String]) {
    let cutoff = now().saturating_sub(STALE_SECS);
    for entry in load_all() {
        if live.contains(&entry.pane_id) || entry.updated >= cutoff {
            continue;
        }
        let _ = std::fs::remove_file(store_dir().join(format!("{}.json", key(&entry.pane_id))));
    }
}

fn show(as_json: bool) {
    let entries = load_all();
    if as_json {
        let list = JsonValue::Array(entries.iter().map(Entry::to_json).collect());
        if let Ok(raw) = list.stringify() {
            println!("{raw}");
        }
        return;
    }

    let labels = disambiguated_workspace_labels();
    let live = live_pane_ids();
    let mut current: Option<String> = None;
    for entry in &entries {
        let Some(restore) = entry.restore() else {
            continue;
        };
        let workspace = entry.workspace.clone().unwrap_or_default();
        if current.as_ref() != Some(&workspace) {
            if current.is_some() {
                println!();
            }
            let label = labels.get(&workspace).cloned().unwrap_or(workspace.clone());
            println!("{label}");
            current = Some(workspace);
        }
        let mark = if live.contains(&entry.pane_id) {
            " "
        } else {
            "-"
        };
        let where_ = entry.cwd.as_deref().map(tilde).unwrap_or_default();
        println!("{mark} {:<10} {where_}", entry.pane_id);
        println!("    {restore}{}", status(entry));
    }
    if current.is_none() {
        println!("nothing recorded yet");
    }
}

/// `(exit 1)` on a failed command; nothing for a resumable agent or a clean run.
fn status(entry: &Entry) -> String {
    if entry.resume().is_some() {
        return String::new();
    }
    if entry.command.is_none() && entry.agent.is_some() {
        return "  (no session recorded)".to_string();
    }
    match (entry.running, entry.exit) {
        (true, _) => "  (running)".to_string(),
        (false, Some(code)) if code != 0 => format!("  (exit {code})"),
        _ => String::new(),
    }
}

fn report(entry: &Entry) {
    if entry.agent.is_some() {
        return;
    }
    let Some(command) = entry.command.as_deref() else {
        return;
    };
    let token = format!("{TOKEN}={}", truncate(command, TOKEN_MAX_LEN));
    herdr_ok(&[
        "pane",
        "report-metadata",
        &entry.pane_id,
        "--source",
        METADATA_SOURCE,
        "--token",
        &token,
    ]);
}

/// Two spaces rooted in the same directory share a label, and back-to-back
/// identical headers read as a bug. Qualify only the ones that collide.
fn disambiguated_workspace_labels() -> HashMap<String, String> {
    let labels = workspace_labels();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for label in labels.values() {
        *seen.entry(label.clone()).or_default() += 1;
    }
    labels
        .into_iter()
        .map(|(id, label)| {
            let label = if seen.get(&label).copied().unwrap_or_default() > 1 {
                format!("{label} ({id})")
            } else {
                label
            };
            (id, label)
        })
        .collect()
}

fn workspace_labels() -> HashMap<String, String> {
    let Some(json) = herdr(&["workspace", "list"]) else {
        return HashMap::new();
    };
    let Some(workspaces) = array(&json, &["result", "workspaces"]) else {
        return HashMap::new();
    };
    workspaces
        .iter()
        .filter_map(|workspace| {
            Some((
                string(workspace, "workspace_id")?,
                string(workspace, "label")?,
            ))
        })
        .collect()
}

fn live_pane_ids() -> Vec<String> {
    herdr(&["pane", "list"])
        .as_ref()
        .and_then(|json| array(json, &["result", "panes"]))
        .map(|panes| panes.iter().filter_map(|pane| string(pane, "pane_id")).collect())
        .unwrap_or_default()
}

// ---- helpers ----

fn pane_id() -> Option<String> {
    std::env::var("HERDR_PANE_ID").ok().filter(|id| !id.is_empty())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_default()
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var(var)
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home().join(fallback))
}

fn tilde(path: &str) -> String {
    let home = home();
    Path::new(path)
        .strip_prefix(&home)
        .map(|rest| format!("~/{}", rest.display()))
        .unwrap_or_else(|_| path.to_string())
}

/// A command line worth remembering: one line, trimmed, and bounded.
fn clean(cmdline: &str) -> Option<String> {
    let flat = cmdline.split_whitespace().collect::<Vec<_>>().join(" ");
    (!flat.is_empty()).then(|| truncate(&flat, COMMAND_MAX_LEN))
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let head: String = value.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}

// ---- herdr CLI ----

fn herdr(args: &[&str]) -> Option<JsonValue> {
    herdr_stdout(args)?.parse().ok()
}

/// `report-metadata` answers with an empty body, so its only verdict is the exit
/// status.
fn herdr_ok(args: &[&str]) -> bool {
    herdr_stdout(args).is_some()
}

fn herdr_stdout(args: &[&str]) -> Option<String> {
    let bin = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let out = Command::new(bin).args(args).output().ok()?;
    out.status.success().then_some(())?;
    String::from_utf8(out.stdout).ok()
}

fn at<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a JsonValue> {
    let mut value = value;
    for key in path {
        value = value.get::<HashMap<String, JsonValue>>()?.get(*key)?;
    }
    Some(value)
}

fn string(value: &JsonValue, key: &str) -> Option<String> {
    at(value, &[key])?.get::<String>().cloned()
}

fn number(value: &JsonValue, key: &str) -> Option<f64> {
    at(value, &[key])?.get::<f64>().copied()
}

fn boolean(value: &JsonValue, key: &str) -> bool {
    at(value, &[key]).and_then(JsonValue::get::<bool>) == Some(&true)
}

fn array<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a Vec<JsonValue>> {
    at(value, path)?.get::<Vec<JsonValue>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_entry(source: &str, kind: &str, value: &str) -> Entry {
        Entry {
            session: Some((source.into(), kind.into(), value.into())),
            ..Entry::default()
        }
    }

    #[test]
    fn resume_matches_herdr_own_invocation() {
        assert_eq!(
            agent_entry("herdr:claude", "id", "abc").resume().as_deref(),
            Some("claude --resume abc")
        );
        assert_eq!(
            agent_entry("herdr:codex", "id", "abc").resume().as_deref(),
            Some("codex resume abc")
        );
        assert_eq!(
            agent_entry("herdr:pi", "path", "/tmp/s").resume().as_deref(),
            Some("pi --session /tmp/s")
        );
    }

    #[test]
    fn resume_ignores_sessions_herdr_did_not_vouch_for() {
        assert_eq!(agent_entry("plugin:x", "id", "abc").resume(), None);
        assert_eq!(agent_entry("herdr:unknown", "id", "abc").resume(), None);
    }

    #[test]
    fn agent_pane_restores_as_the_agent_not_the_shell() {
        let mut entry = agent_entry("herdr:claude", "id", "abc");
        entry.command = Some("vim".into());
        assert_eq!(entry.restore().as_deref(), Some("claude --resume abc"));
    }

    #[test]
    fn agent_without_a_session_still_gets_a_line() {
        let entry = Entry {
            agent: Some("claude".into()),
            ..Entry::default()
        };
        assert_eq!(entry.restore().as_deref(), Some("claude"));
        assert_eq!(status(&entry), "  (no session recorded)");
    }

    #[test]
    fn shell_pane_restores_as_its_last_command() {
        let entry = Entry {
            command: Some("cargo test".into()),
            ..Entry::default()
        };
        assert_eq!(entry.restore().as_deref(), Some("cargo test"));
    }

    #[test]
    fn entry_round_trips_through_the_store_format() {
        let entry = Entry {
            pane_id: "w1:p2".into(),
            workspace: Some("w1".into()),
            tab: Some("w1:t1".into()),
            cwd: Some("/tmp".into()),
            command: Some("ls -la".into()),
            running: false,
            exit: Some(2),
            agent: None,
            session: Some(("herdr:claude".into(), "id".into(), "abc".into())),
            updated: 42,
        };
        let raw = entry.to_json().stringify().expect("stringify");
        assert_eq!(Entry::parse(&raw), Some(entry));
    }

    #[test]
    fn command_lines_are_flattened_and_bounded() {
        assert_eq!(clean("  git   status \n"), Some("git status".into()));
        assert_eq!(clean("   "), None);
        assert_eq!(truncate(&"x".repeat(40), 5), "xxxx…");
    }

    #[test]
    fn pane_ids_are_safe_file_names() {
        assert_eq!(key("w1:p2"), "w1%3Ap2");
        assert_eq!(key("../etc"), "..%2Fetc");
    }
}
