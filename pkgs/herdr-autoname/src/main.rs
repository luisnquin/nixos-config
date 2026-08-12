//! tmux-like automatic tab names for herdr.
//!
//! Modes:
//!   event                 rename the tab the event points at (`HERDR_TAB_ID`)
//!   refresh               sweep every tab; also the startup hook and the manual action
//!   preexec <cmdline>     rename this shell's tab after the command it is about to run
//!   precmd <shell> <cwd>  rename this shell's tab back to its idle name
//!
//! A tab renamed by the user is left alone: state remembers the last name we set
//! per tab, and a label that is neither that name nor herdr's own tab number opts
//! the tab out until it is renamed back.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use tinyjson::JsonValue;

const DEFAULT_MAX_LEN: usize = 20;

const SHELLS: &[&str] = &[
    "ash", "bash", "csh", "dash", "elvish", "fish", "ksh", "nu", "sh", "tcsh", "xonsh", "zsh",
];

/// Programs that prefix the command actually worth naming a tab after.
const WRAPPERS: &[&str] = &[
    "builtin", "command", "doas", "env", "exec", "ionice", "nice", "nohup", "noglob", "stdbuf",
    "sudo", "time", "timeout", "watch",
];

fn main() {
    let cfg = Config::load();
    let namer = Namer {
        cfg,
        store: Store::new(state_dir()),
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("preexec") => namer.rename_current(
            args.get(1)
                .and_then(|cmdline| program_from_cmdline(cmdline, cfg.max_len)),
        ),
        Some("precmd") => namer.rename_current(idle_name(&cfg, args.get(1), args.get(2))),
        Some("refresh") => namer.reconcile(),
        _ => namer.event(),
    }
}

// ---- config ----

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Idle {
    Cwd,
    Shell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Config {
    idle: Idle,
    agent: bool,
    max_len: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle: Idle::Cwd,
            agent: true,
            max_len: DEFAULT_MAX_LEN,
        }
    }
}

impl Config {
    /// `key = value` lines from the config file, then `HERDR_AUTONAME_*` overrides.
    /// The path is fixed instead of `HERDR_PLUGIN_CONFIG_DIR` because the shell
    /// hook runs outside the plugin environment and must read the same file.
    fn load() -> Self {
        let mut cfg = Self::parse(&std::fs::read_to_string(config_path()).unwrap_or_default());
        for (key, var) in [
            ("idle", "HERDR_AUTONAME_IDLE"),
            ("agent", "HERDR_AUTONAME_AGENT"),
            ("max_len", "HERDR_AUTONAME_MAX_LEN"),
        ] {
            if let Ok(value) = std::env::var(var) {
                cfg.set(key, value.trim());
            }
        }
        cfg
    }

    fn parse(text: &str) -> Self {
        let mut cfg = Self::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if let Some((key, value)) = line.split_once('=') {
                cfg.set(key.trim(), value.trim());
            }
        }
        cfg
    }

    fn set(&mut self, key: &str, value: &str) {
        match key {
            "idle" => {
                self.idle = match value {
                    "shell" => Idle::Shell,
                    _ => Idle::Cwd,
                }
            }
            "agent" => self.agent = !matches!(value, "false" | "0" | "no" | "off"),
            "max_len" => {
                if let Ok(len) = value.parse::<usize>() {
                    if len > 0 {
                        self.max_len = len;
                    }
                }
            }
            _ => {}
        }
    }
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| home().join(fallback))
}

fn config_path() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("herdr-autoname/config")
}

fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("herdr-autoname/tabs")
}

// ---- names ----

/// Collapse whitespace, drop control characters, truncate to `max_len`.
fn sanitize(raw: &str, max_len: usize) -> Option<String> {
    let mut name = String::new();
    for word in raw
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
    {
        if !name.is_empty() {
            name.push(' ');
        }
        name.push_str(word);
    }
    let name: String = name.chars().take(max_len).collect();
    let name = name.trim_end();
    (!name.is_empty()).then(|| name.to_string())
}

fn basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

fn is_assignment(word: &str) -> bool {
    let Some((key, _)) = word.split_once('=') else {
        return false;
    };
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Program a command line actually runs: leading `VAR=value` assignments and
/// wrappers such as `sudo` or `env` are skipped, along with their options.
fn program_from_cmdline(cmdline: &str, max_len: usize) -> Option<String> {
    let mut in_wrapper = false;
    for word in cmdline.split_whitespace() {
        if is_assignment(word) || word.starts_with('-') {
            continue;
        }
        // `timeout 5s cmd`, `nice 10 cmd`
        if in_wrapper && word.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let base = basename(word);
        if WRAPPERS.contains(&base) {
            in_wrapper = true;
            continue;
        }
        return sanitize(base, max_len);
    }
    None
}

/// argv0 as the OS reports it: a login shell shows up as `-zsh`.
fn program_from_argv0(argv0: &str, max_len: usize) -> Option<String> {
    sanitize(basename(argv0).trim_start_matches('-'), max_len)
}

fn is_shell(name: &str) -> bool {
    SHELLS.contains(&name)
}

/// Last path segment, with `$HOME` shown as `~`.
fn dir_name(path: &str, max_len: usize) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return sanitize("/", max_len);
    }
    if Path::new(trimmed) == home() {
        return sanitize("~", max_len);
    }
    sanitize(basename(trimmed), max_len)
}

/// Name for a shell sitting at a prompt.
fn idle_name(cfg: &Config, shell: Option<&String>, cwd: Option<&String>) -> Option<String> {
    let from_cwd = (cfg.idle == Idle::Cwd)
        .then(|| cwd.and_then(|cwd| dir_name(cwd, cfg.max_len)))
        .flatten();
    from_cwd.or_else(|| shell.and_then(|shell| program_from_argv0(shell, cfg.max_len)))
}

/// A tab may be auto-named while its label is empty, herdr's own tab number, or
/// the name we set last time.
fn eligible(label: &str, number: Option<f64>, last_set: Option<&str>) -> bool {
    label.is_empty()
        || number.is_some_and(|number| label == number.to_string())
        || last_set == Some(label)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Skip,
    Record,
    Rename,
}

fn decide(label: &str, number: Option<f64>, last_set: Option<&str>, name: &str) -> Action {
    if !eligible(label, number, last_set) {
        Action::Skip
    } else if label != name {
        Action::Rename
    } else if last_set == Some(name) {
        Action::Skip
    } else {
        Action::Record
    }
}

// ---- herdr CLI ----

fn herdr(args: &[&str]) -> Option<JsonValue> {
    let bin = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let out = Command::new(bin).args(args).output().ok()?;
    out.status.success().then_some(())?;
    String::from_utf8(out.stdout).ok()?.parse().ok()
}

fn at<'a>(v: &'a JsonValue, path: &[&str]) -> Option<&'a JsonValue> {
    let mut v = v;
    for key in path {
        v = v.get::<HashMap<String, JsonValue>>()?.get(*key)?;
    }
    Some(v)
}

fn str_at<'a>(v: &'a JsonValue, path: &[&str]) -> Option<&'a str> {
    at(v, path)?.get::<String>().map(String::as_str)
}

fn num_at(v: &JsonValue, path: &[&str]) -> Option<f64> {
    at(v, path)?.get::<f64>().copied()
}

fn is_true(v: &JsonValue, key: &str) -> bool {
    at(v, &[key]).and_then(JsonValue::get::<bool>) == Some(&true)
}

fn array<'a>(v: &'a JsonValue, path: &[&str]) -> Option<&'a Vec<JsonValue>> {
    at(v, path)?.get()
}

/// The pane whose program names a tab: its only pane, or the focused one.
/// Unfocused multi-pane tabs keep whatever name they already have.
fn active_pane<'a>(panes: &'a [JsonValue], tab_id: &str) -> Option<&'a JsonValue> {
    let in_tab: Vec<&JsonValue> = panes
        .iter()
        .filter(|pane| str_at(pane, &["tab_id"]) == Some(tab_id))
        .collect();
    match in_tab.as_slice() {
        [only] => Some(only),
        _ => in_tab.into_iter().find(|pane| is_true(pane, "focused")),
    }
}

/// `HERDR_PLUGIN_EVENT_JSON` holds the whole envelope; `tab.renamed` carries the
/// label herdr just applied.
fn event_label() -> Option<String> {
    label_from_event(&std::env::var("HERDR_PLUGIN_EVENT_JSON").ok()?)
}

fn label_from_event(raw: &str) -> Option<String> {
    let json: JsonValue = raw.parse().ok()?;
    str_at(&json, &["data", "label"]).map(str::to_string)
}

// ---- state: one file per tab, so concurrent writers never clobber each other ----

struct Store {
    dir: PathBuf,
}

impl Store {
    fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Tab ids are shaped like `w1:t2`; percent-encode anything awkward in a file name.
    fn key(tab_id: &str) -> String {
        let mut key = String::with_capacity(tab_id.len());
        for byte in tab_id.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => {
                    key.push(byte as char)
                }
                _ => key.push_str(&format!("%{byte:02X}")),
            }
        }
        key
    }

    fn get(&self, tab_id: &str) -> Option<String> {
        let raw = std::fs::read_to_string(self.dir.join(Self::key(tab_id))).ok()?;
        let name = raw.trim_end_matches('\n');
        (!name.is_empty()).then(|| name.to_string())
    }

    /// 0.1.x kept a single `tabs` file exactly where the directory now lives.
    fn ensure_dir(&self) -> bool {
        if self.dir.is_file() {
            let _ = std::fs::remove_file(&self.dir);
        }
        std::fs::create_dir_all(&self.dir).is_ok()
    }

    fn set(&self, tab_id: &str, name: &str) {
        if !self.ensure_dir() {
            return;
        }
        let path = self.dir.join(Self::key(tab_id));
        let tmp = self
            .dir
            .join(format!("{}.tmp{}", Self::key(tab_id), std::process::id()));
        if std::fs::write(&tmp, name).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    fn clear(&self, tab_id: &str) {
        let _ = std::fs::remove_file(self.dir.join(Self::key(tab_id)));
    }

    /// Drop entries for tabs that no longer exist.
    fn gc(&self, live: &HashSet<String>) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.contains(".tmp") || live.contains(&name) {
                continue;
            }
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

// ---- modes ----

struct Namer {
    cfg: Config,
    store: Store,
}

impl Namer {
    /// Fast path for the shell hook: rename only the tab this shell runs in.
    fn rename_current(&self, name: Option<String>) {
        let (Some(name), Ok(tab_id)) = (name, std::env::var("HERDR_TAB_ID")) else {
            return;
        };
        self.apply(&tab_id, &name);
    }

    /// Plugin event hook. herdr sets `HERDR_TAB_ID` from the event itself, so a
    /// single tab is renamed instead of sweeping the session.
    fn event(&self) {
        match std::env::var("HERDR_PLUGIN_EVENT")
            .unwrap_or_default()
            .as_str()
        {
            "startup" => return self.reconcile(),
            "tab.closed" => {
                if let Ok(tab_id) = std::env::var("HERDR_TAB_ID") {
                    self.store.clear(&tab_id);
                }
                return;
            }
            "tab.renamed" => return self.forget_if_user_renamed(),
            _ => {}
        }
        let Ok(tab_id) = std::env::var("HERDR_TAB_ID") else {
            return self.reconcile();
        };
        let pane = std::env::var("HERDR_PANE_ID")
            .ok()
            .and_then(|pane_id| herdr(&["pane", "get", &pane_id]))
            .and_then(|response| at(&response, &["result", "pane"]).cloned())
            .filter(|pane| str_at(pane, &["tab_id"]) == Some(tab_id.as_str()))
            .or_else(|| self.active_pane_in(&tab_id));
        let Some(name) = pane.as_ref().and_then(|pane| self.name_for_pane(pane)) else {
            return;
        };
        self.apply(&tab_id, &name);
    }

    /// Our own renames come back as `tab.renamed` too; only a label we did not
    /// set opts the tab out.
    fn forget_if_user_renamed(&self) {
        let Ok(tab_id) = std::env::var("HERDR_TAB_ID") else {
            return;
        };
        let Some(last_set) = self.store.get(&tab_id) else {
            return;
        };
        let label = event_label().or_else(|| {
            let tab = herdr(&["tab", "get", &tab_id])?;
            str_at(&tab, &["result", "tab", "label"]).map(str::to_string)
        });
        if label.is_some_and(|label| label != last_set) {
            self.store.clear(&tab_id);
        }
    }

    /// Rename every tab and drop state for tabs that are gone.
    fn reconcile(&self) {
        let (Some(tabs), Some(panes)) = (herdr(&["tab", "list"]), herdr(&["pane", "list"])) else {
            return;
        };
        let empty = Vec::new();
        let tabs = array(&tabs, &["result", "tabs"]).unwrap_or(&empty);
        let panes = array(&panes, &["result", "panes"]).unwrap_or(&empty);

        let mut live = HashSet::new();
        for tab in tabs {
            let Some(tab_id) = str_at(tab, &["tab_id"]) else {
                continue;
            };
            live.insert(Store::key(tab_id));
            let label = str_at(tab, &["label"]).unwrap_or("");
            let number = num_at(tab, &["number"]);
            if !eligible(label, number, self.store.get(tab_id).as_deref()) {
                continue;
            }
            let Some(name) = active_pane(panes, tab_id).and_then(|pane| self.name_for_pane(pane))
            else {
                continue;
            };
            self.apply_known(tab_id, label, number, &name);
        }
        self.store.gc(&live);
    }

    fn active_pane_in(&self, tab_id: &str) -> Option<JsonValue> {
        let response = herdr(&["pane", "list"])?;
        active_pane(array(&response, &["result", "panes"])?, tab_id).cloned()
    }

    /// Detected agent, else the foreground program, else the working directory.
    fn name_for_pane(&self, pane: &JsonValue) -> Option<String> {
        if let Some(name) = self.agent_name(pane) {
            return Some(name);
        }
        let program = str_at(pane, &["pane_id"]).and_then(|pane_id| self.pane_program(pane_id));
        match program {
            Some(program) if !is_shell(&program) => Some(program),
            program => self.pane_cwd(pane).or(program),
        }
    }

    fn agent_name(&self, pane: &JsonValue) -> Option<String> {
        if !self.cfg.agent {
            return None;
        }
        let agent = str_at(pane, &["display_agent"]).or_else(|| str_at(pane, &["agent"]))?;
        sanitize(agent, self.cfg.max_len)
    }

    fn pane_cwd(&self, pane: &JsonValue) -> Option<String> {
        if self.cfg.idle != Idle::Cwd {
            return None;
        }
        let cwd = str_at(pane, &["foreground_cwd"]).or_else(|| str_at(pane, &["cwd"]))?;
        dir_name(cwd, self.cfg.max_len)
    }

    /// Foreground program of a pane: the process-group leader's command.
    fn pane_program(&self, pane_id: &str) -> Option<String> {
        let response = herdr(&["pane", "process-info", "--pane", pane_id])?;
        let info = at(&response, &["result", "process_info"])?;
        let leader = num_at(info, &["foreground_process_group_id"])?;
        let procs = array(info, &["foreground_processes"])?;
        let proc = procs
            .iter()
            .find(|proc| num_at(proc, &["pid"]) == Some(leader))?;
        if let Some(name) = str_at(proc, &["cmdline"])
            .and_then(|cmdline| program_from_cmdline(cmdline, self.cfg.max_len))
        {
            return Some(name);
        }
        let argv0 = str_at(proc, &["argv0"])
            .or_else(|| {
                array(proc, &["argv"])?
                    .first()?
                    .get::<String>()
                    .map(String::as_str)
            })
            .or_else(|| str_at(proc, &["name"]))?;
        program_from_argv0(argv0, self.cfg.max_len)
    }

    fn apply(&self, tab_id: &str, name: &str) {
        if self.store.get(tab_id).as_deref() == Some(name) {
            return;
        }
        let Some(tab) = herdr(&["tab", "get", tab_id]) else {
            return;
        };
        let label = str_at(&tab, &["result", "tab", "label"]).unwrap_or("");
        let number = num_at(&tab, &["result", "tab", "number"]);
        self.apply_known(tab_id, label, number, name);
    }

    /// State is written before the rename so the `tab.renamed` hook we trigger
    /// recognises the label as ours.
    fn apply_known(&self, tab_id: &str, label: &str, number: Option<f64>, name: &str) {
        match decide(label, number, self.store.get(tab_id).as_deref(), name) {
            Action::Skip => {}
            Action::Record => self.store.set(tab_id, name),
            Action::Rename => {
                self.store.set(tab_id, name);
                herdr(&["tab", "rename", tab_id, name]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: usize = 20;

    #[test]
    fn program_from_cmdline_takes_basename_of_first_word() {
        assert_eq!(
            program_from_cmdline("nvim src/main.rs", MAX).as_deref(),
            Some("nvim")
        );
        assert_eq!(
            program_from_cmdline("/usr/bin/git status", MAX).as_deref(),
            Some("git")
        );
        assert_eq!(program_from_cmdline("  ", MAX), None);
    }

    #[test]
    fn program_from_cmdline_skips_wrappers_and_assignments() {
        for cmdline in [
            "sudo nvim /etc/hosts",
            "env RUST_LOG=debug nvim",
            "RUST_LOG=debug nvim",
            "PATH=/usr/bin:/bin nvim",
            "timeout 30 nvim",
            "nohup nice -n 10 nvim",
            "command nvim",
        ] {
            assert_eq!(
                program_from_cmdline(cmdline, MAX).as_deref(),
                Some("nvim"),
                "{cmdline}"
            );
        }
    }

    #[test]
    fn program_from_argv0_strips_login_dash() {
        assert_eq!(program_from_argv0("-zsh", MAX).as_deref(), Some("zsh"));
        assert_eq!(program_from_argv0("/bin/zsh", MAX).as_deref(), Some("zsh"));
    }

    #[test]
    fn sanitize_collapses_whitespace_and_truncates() {
        assert_eq!(
            sanitize("  git   status ", MAX).as_deref(),
            Some("git status")
        );
        assert_eq!(sanitize("a\u{1b}[0mb", MAX).as_deref(), Some("a [0mb"));
        assert_eq!(
            sanitize("x".repeat(40).as_str(), 4).as_deref(),
            Some("xxxx")
        );
        assert_eq!(sanitize("\t\n", MAX), None);
    }

    #[test]
    fn user_renamed_tabs_are_not_eligible() {
        assert!(eligible("2", Some(2.0), None)); // herdr's own tab number
        assert!(eligible("", Some(2.0), None));
        assert!(eligible("nvim", Some(2.0), Some("nvim"))); // our own name
        assert!(!eligible("my tab", Some(2.0), Some("nvim"))); // user rename
        assert!(!eligible("my tab", Some(2.0), None));
        assert!(!eligible("2024", Some(3.0), None)); // a digit label the user chose
    }

    #[test]
    fn dir_name_shortens_home_and_paths() {
        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(
            dir_name("/home/x/.dotfiles/", MAX).as_deref(),
            Some(".dotfiles")
        );
        assert_eq!(dir_name("/", MAX).as_deref(), Some("/"));
        if !home.is_empty() {
            assert_eq!(dir_name(&home, MAX).as_deref(), Some("~"));
        }
    }

    #[test]
    fn idle_name_prefers_cwd_then_shell() {
        let cwd = "/srv/api".to_string();
        let shell = "-zsh".to_string();
        let cwd_cfg = Config::default();
        let shell_cfg = Config {
            idle: Idle::Shell,
            ..Config::default()
        };
        assert_eq!(
            idle_name(&cwd_cfg, Some(&shell), Some(&cwd)).as_deref(),
            Some("api")
        );
        assert_eq!(
            idle_name(&shell_cfg, Some(&shell), Some(&cwd)).as_deref(),
            Some("zsh")
        );
        assert_eq!(
            idle_name(&cwd_cfg, Some(&shell), None).as_deref(),
            Some("zsh")
        );
    }

    #[test]
    fn config_reads_keys_and_ignores_junk() {
        let mut cfg = Config::default();
        cfg.set("idle", "shell");
        cfg.set("agent", "false");
        cfg.set("max_len", "8");
        cfg.set("max_len", "0");
        cfg.set("nope", "1");
        assert_eq!(
            cfg,
            Config {
                idle: Idle::Shell,
                agent: false,
                max_len: 8
            }
        );
    }

    #[test]
    fn active_pane_prefers_single_pane_then_focused() {
        let panes: Vec<JsonValue> = [
            r#"{"pane_id": "p1", "tab_id": "t1", "focused": false}"#,
            r#"{"pane_id": "p2", "tab_id": "t2", "focused": false}"#,
            r#"{"pane_id": "p3", "tab_id": "t2", "focused": true}"#,
            r#"{"pane_id": "p4", "tab_id": "t3", "focused": false}"#,
            r#"{"pane_id": "p5", "tab_id": "t3", "focused": false}"#,
        ]
        .iter()
        .map(|pane| pane.parse().unwrap())
        .collect();
        let id = |pane: Option<&JsonValue>| {
            pane.and_then(|p| str_at(p, &["pane_id"]))
                .map(str::to_string)
        };
        assert_eq!(id(active_pane(&panes, "t1")).as_deref(), Some("p1"));
        assert_eq!(id(active_pane(&panes, "t2")).as_deref(), Some("p3"));
        assert_eq!(id(active_pane(&panes, "t3")), None);
    }

    #[test]
    fn store_roundtrips_and_collects_dead_tabs() {
        let dir = std::env::temp_dir().join(format!("herdr-autoname-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::new(dir.clone());

        // 0.1.x state file sitting where the directory belongs
        std::fs::write(&dir, "w1:t1\tnvim\n").unwrap();

        store.set("w1:t1", "nvim");
        store.set("w1:t2", "cargo");
        assert_eq!(store.get("w1:t1").as_deref(), Some("nvim"));
        assert_eq!(store.get("missing"), None);

        store.clear("w1:t1");
        assert_eq!(store.get("w1:t1"), None);

        store.gc(&HashSet::new());
        assert_eq!(store.get("w1:t2"), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decide_renames_records_or_backs_off() {
        let number = Some(2.0);
        // fresh tab still labelled with its number
        assert_eq!(decide("2", number, None, "nvim"), Action::Rename);
        // label already ours but state was lost
        assert_eq!(decide("nvim", number, None, "nvim"), Action::Skip);
        assert_eq!(decide("nvim", number, Some("nvim"), "nvim"), Action::Skip);
        // our own tab, new program
        assert_eq!(
            decide("nvim", number, Some("nvim"), "cargo"),
            Action::Rename
        );
        // herdr reset the label under us
        assert_eq!(decide("2", number, Some("nvim"), "nvim"), Action::Rename);
        // user rename wins
        assert_eq!(decide("logs", number, Some("nvim"), "cargo"), Action::Skip);
        assert_eq!(decide("logs", number, None, "cargo"), Action::Skip);
    }

    #[test]
    fn decide_handles_tabs_without_a_number() {
        // a fresh tab herdr has not numbered yet
        assert_eq!(decide("", None, None, "nvim"), Action::Rename);
        assert_eq!(decide("nvim", None, Some("nvim"), "nvim"), Action::Skip);
        assert_eq!(decide("logs", None, None, "nvim"), Action::Skip);
    }

    #[test]
    fn config_parses_file_with_comments() {
        let cfg = Config::parse(
            "# tab names\nidle = shell\nagent=no  # herdr detects it anyway\n\nmax_len = 12\nbogus\n",
        );
        assert_eq!(
            cfg,
            Config {
                idle: Idle::Shell,
                agent: false,
                max_len: 12
            }
        );
        assert_eq!(Config::parse(""), Config::default());
    }

    #[test]
    fn label_from_event_reads_tab_renamed_payload() {
        let raw = r#"{"event":"tab.renamed","data":{"type":"tab_renamed","tab_id":"w1:t2","workspace_id":"w1","label":"logs"}}"#;
        assert_eq!(label_from_event(raw).as_deref(), Some("logs"));
        assert_eq!(label_from_event(r#"{"data":{}}"#), None);
        assert_eq!(label_from_event("not json"), None);
    }

    #[test]
    fn pane_names_prefer_agent_then_cwd() {
        let namer = |cfg| Namer {
            cfg,
            store: Store::new(PathBuf::from("/nonexistent")),
        };
        let pane: JsonValue = r#"{
            "pane_id": "w1:p1", "tab_id": "w1:t1", "focused": true,
            "cwd": "/home/x/src/api", "foreground_cwd": "/home/x/src/api/crates",
            "agent": "claude", "display_agent": "claude sonnet"
        }"#
        .parse()
        .unwrap();

        let default = namer(Config::default());
        assert_eq!(default.agent_name(&pane).as_deref(), Some("claude sonnet"));
        assert_eq!(default.pane_cwd(&pane).as_deref(), Some("crates"));

        let no_agent = namer(Config {
            agent: false,
            ..Config::default()
        });
        assert_eq!(no_agent.agent_name(&pane), None);

        let shell_idle = namer(Config {
            idle: Idle::Shell,
            ..Config::default()
        });
        assert_eq!(shell_idle.pane_cwd(&pane), None);

        let bare: JsonValue = r#"{"pane_id": "w1:p2", "cwd": "/srv"}"#.parse().unwrap();
        assert_eq!(default.agent_name(&bare), None);
        assert_eq!(default.pane_cwd(&bare).as_deref(), Some("srv"));
    }

    #[test]
    fn tab_json_fields_drive_eligibility() {
        let tab: JsonValue = r#"{"tab_id":"w1:t3","label":"3","number":3,"focused":false}"#
            .parse()
            .unwrap();
        let label = str_at(&tab, &["label"]).unwrap();
        let number = num_at(&tab, &["number"]);
        assert_eq!(number, Some(3.0));
        assert!(eligible(label, number, None));
        assert!(!is_true(&tab, "focused"));
    }

    #[test]
    fn store_keys_are_safe_file_names() {
        assert_eq!(Store::key("w1:t2"), "w1%3At2");
        assert_eq!(Store::key("../escape"), "..%2Fescape");
    }
}
