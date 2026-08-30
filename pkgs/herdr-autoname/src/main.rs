//! tmux-like automatic tab and space names for herdr.
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
//!
//! Spaces are named from the active tab's pane. herdr derives its own space label
//! from the *first* tab's root pane, so a background pane decides the name of the
//! space you are looking at; setting a custom name pins it to the visible pane.
//!
//! Icons are reported as metadata, never folded into a name: a name doubles as
//! the opt-out marker.

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

const METADATA_SOURCE: &str = "herdr-autoname";

const MAX_ICON_CHARS: usize = 2;

/// Marker file at the repository root, `icon.<key>` override key, glyph.
///
/// First match wins, so the order runs framework, then language, then packaging.
/// `flake.nix` sits near the bottom: nearly every repository here carries one,
/// and a snowflake on all of them tells the spaces apart from nothing.
const PROJECT_ICONS: &[(&str, &str, &str)] = &[
    ("app.config.ts", "expo", "\u{e7ba}"),
    ("app.config.js", "expo", "\u{e7ba}"),
    ("encore.app", "encore", "\u{f233}"),
    ("svelte.config.js", "svelte", "\u{e697}"),
    ("nuxt.config.ts", "vue", "\u{e6a0}"),
    ("Chart.yaml", "helm", "\u{f10fe}"),
    ("main.tf", "terraform", "\u{e69a}"),
    ("Dockerfile", "docker", "\u{f308}"),
    ("compose.yaml", "docker", "\u{f308}"),
    ("docker-compose.yml", "docker", "\u{f308}"),
    ("Cargo.toml", "rust", "\u{e7a8}"),
    ("go.mod", "go", "\u{e627}"),
    ("build.zig", "zig", "\u{e6a9}"),
    ("pyproject.toml", "python", "\u{e73c}"),
    ("requirements.txt", "python", "\u{e73c}"),
    ("mix.exs", "elixir", "\u{e62d}"),
    ("Gemfile", "ruby", "\u{e739}"),
    ("composer.json", "php", "\u{e73d}"),
    ("Package.swift", "swift", "\u{e755}"),
    ("build.gradle.kts", "kotlin", "\u{e634}"),
    ("build.gradle", "java", "\u{e738}"),
    ("pom.xml", "java", "\u{e738}"),
    ("stack.yaml", "haskell", "\u{e777}"),
    ("dune-project", "ocaml", "\u{e67a}"),
    ("build.sbt", "scala", "\u{e737}"),
    ("elm.json", "elm", "\u{e62c}"),
    (".luarc.json", "lua", "\u{e620}"),
    ("CMakeLists.txt", "cpp", "\u{e646}"),
    ("tsconfig.json", "typescript", "\u{e628}"),
    ("jsconfig.json", "javascript", "\u{e781}"),
    ("package.json", "node", "\u{e718}"),
    ("flake.nix", "nix", "\u{f313}"),
    ("default.nix", "nix", "\u{f313}"),
    ("shell.nix", "nix", "\u{f313}"),
];

/// Nerd Fonts draws neither brand, so `claude` and `codex` are private-use
/// glyphs patched into the system font by `system/modules/desktop/fonts`.
const AGENT_ICONS: &[(&str, &str)] = &[
    ("claude", "\u{e9fb}"),
    ("codex", "\u{e9fa}"),
    ("opencode", "\u{f489}"),
    ("pi", "\u{3c0}"),
];

const ICON_REPO: &str = "\u{e702}";
const ICON_DIR: &str = "\u{f07b}";
const ICON_AGENT: &str = "\u{f06a9}";

fn main() {
    let namer = Namer {
        cfg: Config::load(),
        store: Store::new(tabs_dir()),
        spaces: Store::new(spaces_dir()),
        icons: Store::new(icons_dir()),
    };
    let max_len = namer.cfg.max_len;
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("preexec") => namer.rename_current(
            args.get(1)
                .and_then(|cmdline| program_from_cmdline(cmdline, max_len)),
        ),
        Some("precmd") => {
            namer.rename_current(idle_name(&namer.cfg, args.get(1), args.get(2)));
            namer.rename_current_space(args.get(2).map(String::as_str));
        }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SpaceName {
    Repo,
    Cwd,
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Config {
    idle: Idle,
    agent: bool,
    icons: bool,
    max_len: usize,
    space: SpaceName,
    icon_overrides: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle: Idle::Cwd,
            agent: true,
            icons: true,
            max_len: DEFAULT_MAX_LEN,
            space: SpaceName::Repo,
            icon_overrides: HashMap::new(),
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
            ("icons", "HERDR_AUTONAME_ICONS"),
            ("max_len", "HERDR_AUTONAME_MAX_LEN"),
            ("space", "HERDR_AUTONAME_SPACE"),
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
        if let Some(name) = key.strip_prefix("icon.") {
            return self.set_icon(name, value);
        }
        match key {
            "idle" => {
                self.idle = match value {
                    "shell" => Idle::Shell,
                    _ => Idle::Cwd,
                }
            }
            "agent" => self.agent = !matches!(value, "false" | "0" | "no" | "off"),
            "icons" => self.icons = !matches!(value, "false" | "0" | "no" | "off"),
            "space" => {
                self.space = match value {
                    "cwd" => SpaceName::Cwd,
                    "false" | "0" | "no" | "off" => SpaceName::Off,
                    _ => SpaceName::Repo,
                }
            }
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

    /// An empty value drops back to the built-in glyph.
    fn set_icon(&mut self, name: &str, glyph: &str) {
        let name = name.trim().to_lowercase();
        if name.is_empty() {
            return;
        }
        let glyph: String = glyph
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_ICON_CHARS)
            .collect();
        if glyph.is_empty() {
            self.icon_overrides.remove(&name);
        } else {
            self.icon_overrides.insert(name, glyph);
        }
    }

    fn icon_or(&self, key: &str, fallback: &str) -> String {
        self.icon_overrides
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    /// `display_agent` may carry a model too, so a key matches the first word.
    fn agent_icon(&self, agent: &str) -> String {
        let key = agent
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_lowercase();
        let glyph = AGENT_ICONS
            .iter()
            .find(|(name, _)| *name == key)
            .map_or(ICON_AGENT, |(_, glyph)| glyph);
        self.icon_or(&key, glyph)
    }

    fn dir_icon(&self, cwd: &str) -> String {
        let Some(root) = repo_root(cwd) else {
            return self.icon_or("dir", ICON_DIR);
        };
        PROJECT_ICONS
            .iter()
            .find(|(marker, ..)| root.join(marker).exists())
            .map_or_else(
                || self.icon_or("git", ICON_REPO),
                |(_, kind, glyph)| self.icon_or(kind, glyph),
            )
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

fn tabs_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("herdr-autoname/tabs")
}

fn spaces_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("herdr-autoname/spaces")
}

/// Keyed by pane and by workspace id — the two shapes never collide.
fn icons_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("herdr-autoname/icons")
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

/// Innermost ancestor holding a `.git` entry; a linked worktree keeps a file there.
fn repo_root(cwd: &str) -> Option<PathBuf> {
    let mut dir = Path::new(cwd);
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
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

/// A space label is always cwd-derived, so herdr's own name is indistinguishable
/// from a user's. An untouched space is adopted; after that only our own name is
/// ours to replace, and a foreign label opts the space out via the off marker.
fn space_eligible(label: &str, last_set: Option<&str>) -> bool {
    last_set.is_none() || last_set == Some(label)
}

/// Spaces cut from the same repository all resolve to one name, which leaves the
/// switcher listing the same word four times. Twins take a number in the order
/// herdr lists them, the first keeping the bare name: the number appears only
/// where the ambiguity does, and stays put when a twin is renamed away.
fn number_twins(bases: Vec<(String, String)>, max_len: usize) -> Vec<(String, String)> {
    let mut twins: HashMap<&str, usize> = HashMap::new();
    for (_, base) in &bases {
        *twins.entry(base.as_str()).or_default() += 1;
    }
    let alone: HashSet<String> = twins
        .into_iter()
        .filter(|(_, count)| *count < 2)
        .map(|(base, _)| base.to_owned())
        .collect();

    let mut seen: HashMap<String, usize> = HashMap::new();
    bases
        .into_iter()
        .map(|(ws_id, base)| {
            if alone.contains(&base) {
                return (ws_id, base);
            }
            let index = seen.entry(base.clone()).or_default();
            *index += 1;
            let name = match *index {
                1 => base,
                index => numbered(&base, index, max_len),
            };
            (ws_id, name)
        })
        .collect()
}

/// The number has to fit inside the same budget as the name it qualifies.
fn numbered(base: &str, index: usize, max_len: usize) -> String {
    let suffix = format!(" {index}");
    let room = max_len.saturating_sub(suffix.chars().count());
    let head: String = base.chars().take(room).collect();
    format!("{}{suffix}", head.trim_end())
}

/// Whether a label is a name wearing its twin number, so the cheap paths can tell
/// a stale name from one that is merely qualified.
fn is_variant_of(label: &str, base: &str) -> bool {
    if label == base {
        return true;
    }
    label
        .strip_prefix(base)
        .and_then(|rest| rest.strip_prefix(' '))
        .is_some_and(|index| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Skip,
    Record,
    Rename,
}

fn decide(eligible: bool, label: &str, last_set: Option<&str>, name: &str) -> Action {
    if !eligible {
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

/// Directory a space is named and iconified after. `cwd` is preferred over
/// `foreground_cwd`: a child process that chdirs elsewhere must not drag the
/// whole space along with it.
fn space_cwd(pane: &JsonValue) -> Option<&str> {
    str_at(pane, &["cwd"]).or_else(|| str_at(pane, &["foreground_cwd"]))
}

/// The pane a space is named after: the focused pane of its active tab, else the
/// first one. Unlike a tab name, a space name is better stale than missing.
fn space_pane<'a>(panes: &'a [JsonValue], tab_id: &str) -> Option<&'a JsonValue> {
    let mut in_tab = panes
        .iter()
        .filter(|pane| str_at(pane, &["tab_id"]) == Some(tab_id))
        .peekable();
    let first = in_tab.peek().copied();
    in_tab.find(|pane| is_true(pane, "focused")).or(first)
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

    /// Marker for an id we must never rename again, kept beside its name entry.
    fn off_key(id: &str) -> String {
        format!("{}.off", Self::key(id))
    }

    fn is_off(&self, id: &str) -> bool {
        self.dir.join(Self::off_key(id)).exists()
    }

    fn set_off(&self, id: &str) {
        if self.ensure_dir() {
            let _ = std::fs::write(self.dir.join(Self::off_key(id)), "");
        }
    }

    fn clear_off(&self, id: &str) {
        let _ = std::fs::remove_file(self.dir.join(Self::off_key(id)));
    }

    /// Drop entries for tabs or spaces that no longer exist.
    fn gc(&self, live: &HashSet<String>) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let key = name.strip_suffix(".off").unwrap_or(&name);
            if name.contains(".tmp") || live.contains(key) {
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
    spaces: Store,
    icons: Store,
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
            "workspace.closed" => {
                if let Ok(ws_id) = std::env::var("HERDR_WORKSPACE_ID") {
                    self.spaces.clear(&ws_id);
                    self.spaces.clear_off(&ws_id);
                }
                // Numbers are positions among twins, so the survivors move up.
                return self.sweep_spaces();
            }
            "workspace.renamed" => return self.forget_space_if_user_renamed(),
            // No return: the tab still needs a name for whatever is left.
            "pane.exited" => {
                if let Ok(pane_id) = std::env::var("HERDR_PANE_ID") {
                    self.icons.clear(&pane_id);
                }
            }
            _ => {}
        }
        let Ok(tab_id) = std::env::var("HERDR_TAB_ID") else {
            // workspace.created and workspace.focused carry no tab.
            return match std::env::var("HERDR_WORKSPACE_ID") {
                Ok(ws_id) => self.rename_space(&ws_id, None),
                Err(_) => self.reconcile(),
            };
        };
        let pane = std::env::var("HERDR_PANE_ID")
            .ok()
            .and_then(|pane_id| herdr(&["pane", "get", &pane_id]))
            .and_then(|response| at(&response, &["result", "pane"]).cloned())
            .filter(|pane| str_at(pane, &["tab_id"]) == Some(tab_id.as_str()))
            .or_else(|| self.active_pane_in(&tab_id));
        if let Some(pane) = pane.as_ref() {
            self.report_pane_icon(pane);
        }
        if let Ok(ws_id) = std::env::var("HERDR_WORKSPACE_ID") {
            self.rename_space(&ws_id, pane.as_ref());
        }
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

    /// Our own space renames come back as `workspace.renamed` too. A foreign label
    /// opts the space out for good; renaming it back to the automatic name opts in.
    fn forget_space_if_user_renamed(&self) {
        let Ok(ws_id) = std::env::var("HERDR_WORKSPACE_ID") else {
            return;
        };
        let Some(label) = event_label().or_else(|| {
            let ws = herdr(&["workspace", "get", &ws_id])?;
            str_at(&ws, &["result", "workspace", "label"]).map(str::to_string)
        }) else {
            return;
        };
        if self.spaces.get(&ws_id).as_deref() == Some(label.as_str()) {
            return;
        }
        if self.space_name(&ws_id, None).as_deref() == Some(label.as_str()) {
            self.spaces.clear_off(&ws_id);
            self.spaces.set(&ws_id, &label);
            return;
        }
        self.spaces.clear(&ws_id);
        self.spaces.set_off(&ws_id);
    }

    /// Rename every tab and space, and drop state for the ones that are gone.
    fn reconcile(&self) {
        let (Some(tabs), Some(panes)) = (herdr(&["tab", "list"]), herdr(&["pane", "list"])) else {
            return;
        };
        let empty = Vec::new();
        let tabs = array(&tabs, &["result", "tabs"]).unwrap_or(&empty);
        let panes = array(&panes, &["result", "panes"]).unwrap_or(&empty);

        let mut icons_live = HashSet::new();
        for pane in panes {
            self.report_pane_icon(pane);
            if let Some(pane_id) = str_at(pane, &["pane_id"]) {
                icons_live.insert(Store::key(pane_id));
            }
        }

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
        self.reconcile_spaces(panes, icons_live);
    }

    /// `icons_live` arrives holding the pane keys: one store holds both.
    fn reconcile_spaces(&self, panes: &[JsonValue], mut icons_live: HashSet<String>) {
        let Some(response) = herdr(&["workspace", "list"]) else {
            return;
        };
        let empty = Vec::new();
        let workspaces = array(&response, &["result", "workspaces"]).unwrap_or(&empty);

        let mut live = HashSet::new();
        for ws in workspaces {
            let Some(ws_id) = str_at(ws, &["workspace_id"]) else {
                continue;
            };
            live.insert(Store::key(ws_id));
            icons_live.insert(Store::key(ws_id));
            let cwd = str_at(ws, &["active_tab_id"])
                .and_then(|tab_id| space_pane(panes, tab_id))
                .and_then(space_cwd);
            if let Some(cwd) = cwd {
                self.report_space_icon(ws_id, cwd);
            }
        }
        self.name_spaces(panes, workspaces);
        self.spaces.gc(&live);
        self.icons.gc(&icons_live);
    }

    /// Re-number the spaces on their own, for the events that change who is a
    /// twin of whom without touching a single tab.
    fn sweep_spaces(&self) {
        let (Some(panes), Some(workspaces)) =
            (herdr(&["pane", "list"]), herdr(&["workspace", "list"]))
        else {
            return;
        };
        let empty = Vec::new();
        self.name_spaces(
            array(&panes, &["result", "panes"]).unwrap_or(&empty),
            array(&workspaces, &["result", "workspaces"]).unwrap_or(&empty),
        );
    }

    /// A twin number is only knowable from the whole list, so every space is
    /// named in one pass rather than each on its own.
    fn name_spaces(&self, panes: &[JsonValue], workspaces: &[JsonValue]) {
        for (ws_id, name) in number_twins(self.space_bases(panes, workspaces, None), self.cfg.max_len)
        {
            let label = workspaces
                .iter()
                .find(|ws| str_at(ws, &["workspace_id"]) == Some(ws_id.as_str()))
                .and_then(|ws| str_at(ws, &["label"]))
                .unwrap_or("");
            self.apply_space_known(&ws_id, label, &name);
        }
    }

    /// The name every auto-named space would carry before twins are numbered.
    /// `known` substitutes a name already computed elsewhere, so the caller that
    /// has one does not pay for it twice.
    fn space_bases(
        &self,
        panes: &[JsonValue],
        workspaces: &[JsonValue],
        known: Option<(&str, &str)>,
    ) -> Vec<(String, String)> {
        if self.cfg.space == SpaceName::Off {
            return Vec::new();
        }
        workspaces
            .iter()
            .filter_map(|ws| {
                let ws_id = str_at(ws, &["workspace_id"])?;
                if self.spaces.is_off(ws_id) {
                    return None;
                }
                if let Some((known_id, name)) = known {
                    if known_id == ws_id {
                        return Some((ws_id.to_owned(), name.to_owned()));
                    }
                }
                let pane =
                    str_at(ws, &["active_tab_id"]).and_then(|tab_id| space_pane(panes, tab_id))?;
                Some((ws_id.to_owned(), self.name_for_space(pane)?))
            })
            .collect()
    }

    /// The number `base` earns once the rest of the session is taken into
    /// account. Only worth asking when the base itself changed.
    fn qualify(&self, ws_id: &str, base: &str) -> String {
        let (Some(panes), Some(workspaces)) =
            (herdr(&["pane", "list"]), herdr(&["workspace", "list"]))
        else {
            return base.to_owned();
        };
        let empty = Vec::new();
        let bases = self.space_bases(
            array(&panes, &["result", "panes"]).unwrap_or(&empty),
            array(&workspaces, &["result", "workspaces"]).unwrap_or(&empty),
            Some((ws_id, base)),
        );
        number_twins(bases, self.cfg.max_len)
            .into_iter()
            .find(|(id, _)| id == ws_id)
            .map(|(_, name)| name)
            .unwrap_or_else(|| base.to_owned())
    }

    /// A name already carrying the right number is left alone, which keeps the
    /// steady state off the twin lookup entirely.
    fn settled(&self, ws_id: &str, base: &str) -> String {
        match self.spaces.get(ws_id) {
            Some(stored) if is_variant_of(&stored, base) => stored,
            _ => self.qualify(ws_id, base),
        }
    }

    fn active_pane_in(&self, tab_id: &str) -> Option<JsonValue> {
        let response = herdr(&["pane", "list"])?;
        active_pane(array(&response, &["result", "panes"])?, tab_id).cloned()
    }

    fn space_pane_in(&self, tab_id: &str) -> Option<JsonValue> {
        let response = herdr(&["pane", "list"])?;
        space_pane(array(&response, &["result", "panes"])?, tab_id).cloned()
    }

    /// Repository holding the pane, else its directory.
    fn name_for_space(&self, pane: &JsonValue) -> Option<String> {
        self.name_from_cwd(space_cwd(pane)?)
    }

    /// State is written after the call, so a failed report is retried.
    fn report_icon(&self, scope: &str, id: &str, icon: &str) {
        if self.icons.get(id).as_deref() == Some(icon) {
            return;
        }
        let token = format!("icon={icon}");
        let reported = herdr_ok(&[
            scope,
            "report-metadata",
            id,
            "--source",
            METADATA_SOURCE,
            "--token",
            &token,
        ]);
        if reported {
            self.icons.set(id, icon);
        }
    }

    fn report_pane_icon(&self, pane: &JsonValue) {
        if !self.cfg.icons {
            return;
        }
        let (Some(pane_id), Some(agent)) = (
            str_at(pane, &["pane_id"]),
            str_at(pane, &["agent"]).or_else(|| str_at(pane, &["display_agent"])),
        ) else {
            return;
        };
        self.report_icon("pane", pane_id, &self.cfg.agent_icon(agent));
    }

    fn report_space_icon(&self, ws_id: &str, cwd: &str) {
        if !self.cfg.icons {
            return;
        }
        self.report_icon("workspace", ws_id, &self.cfg.dir_icon(cwd));
    }

    fn name_from_cwd(&self, cwd: &str) -> Option<String> {
        match self.cfg.space {
            SpaceName::Off => None,
            SpaceName::Cwd => dir_name(cwd, self.cfg.max_len),
            SpaceName::Repo => repo_root(cwd)
                .and_then(|root| dir_name(&root.to_string_lossy(), self.cfg.max_len))
                .or_else(|| dir_name(cwd, self.cfg.max_len)),
        }
    }

    /// The pane the event carries when it sits in the active tab, else that
    /// tab's own pane.
    fn space_pane_for(&self, ws_id: &str, hint: Option<&JsonValue>) -> Option<JsonValue> {
        let ws = herdr(&["workspace", "get", ws_id])?;
        let tab_id = str_at(&ws, &["result", "workspace", "active_tab_id"])?;
        hint.filter(|pane| str_at(pane, &["tab_id"]) == Some(tab_id))
            .cloned()
            .or_else(|| self.space_pane_in(tab_id))
    }

    fn space_name(&self, ws_id: &str, hint: Option<&JsonValue>) -> Option<String> {
        let base = self.name_for_space(&self.space_pane_for(ws_id, hint)?)?;
        Some(self.qualify(ws_id, &base))
    }

    fn rename_space(&self, ws_id: &str, hint: Option<&JsonValue>) {
        let naming = self.cfg.space != SpaceName::Off && !self.spaces.is_off(ws_id);
        if !naming && !self.cfg.icons {
            return;
        }
        let Some(pane) = self.space_pane_for(ws_id, hint) else {
            return;
        };
        if let Some(cwd) = space_cwd(&pane) {
            self.report_space_icon(ws_id, cwd);
        }
        let Some(base) = naming.then(|| self.name_for_space(&pane)).flatten() else {
            return;
        };
        let name = self.settled(ws_id, &base);
        self.apply_space(ws_id, &name);
    }

    /// Shell hook path: this pane's cwd already names the space when the pane sits
    /// in the active tab, so the steady state costs no API call at all.
    fn rename_current_space(&self, cwd: Option<&str>) {
        let (Some(cwd), Ok(ws_id), Ok(tab_id)) = (
            cwd,
            std::env::var("HERDR_WORKSPACE_ID"),
            std::env::var("HERDR_TAB_ID"),
        ) else {
            return;
        };
        let naming = self.cfg.space != SpaceName::Off && !self.spaces.is_off(&ws_id);
        let base = naming.then(|| self.name_from_cwd(cwd)).flatten();
        let icon = self.cfg.icons.then(|| self.cfg.dir_icon(cwd));
        // A stored name that is only this one wearing a twin number is not stale,
        // which is what keeps the shell hook off the session-wide lookup.
        let stale_name = matches!(
            &base,
            Some(base) if !self.spaces.get(&ws_id).is_some_and(|stored| is_variant_of(&stored, base))
        );
        let stale_icon = matches!(
            &icon,
            Some(icon) if self.icons.get(&ws_id).as_deref() != Some(icon.as_str())
        );
        if !stale_name && !stale_icon {
            return;
        }
        let Some(ws) = herdr(&["workspace", "get", &ws_id]) else {
            return;
        };
        if str_at(&ws, &["result", "workspace", "active_tab_id"]) != Some(tab_id.as_str()) {
            return;
        }
        if let Some(icon) = icon {
            self.report_icon("workspace", &ws_id, &icon);
        }
        let Some(base) = base.filter(|_| stale_name) else {
            return;
        };
        let name = self.qualify(&ws_id, &base);
        let label = str_at(&ws, &["result", "workspace", "label"]).unwrap_or("");
        self.apply_space_known(&ws_id, label, &name);
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
        let last_set = self.store.get(tab_id);
        match decide(
            eligible(label, number, last_set.as_deref()),
            label,
            last_set.as_deref(),
            name,
        ) {
            Action::Skip => {}
            Action::Record => self.store.set(tab_id, name),
            Action::Rename => {
                self.store.set(tab_id, name);
                herdr(&["tab", "rename", tab_id, name]);
            }
        }
    }

    fn apply_space(&self, ws_id: &str, name: &str) {
        if self.spaces.get(ws_id).as_deref() == Some(name) {
            return;
        }
        let Some(ws) = herdr(&["workspace", "get", ws_id]) else {
            return;
        };
        let label = str_at(&ws, &["result", "workspace", "label"]).unwrap_or("");
        self.apply_space_known(ws_id, label, name);
    }

    fn apply_space_known(&self, ws_id: &str, label: &str, name: &str) {
        let last_set = self.spaces.get(ws_id);
        match decide(
            space_eligible(label, last_set.as_deref()),
            label,
            last_set.as_deref(),
            name,
        ) {
            Action::Skip => {}
            Action::Record => self.spaces.set(ws_id, name),
            Action::Rename => {
                self.spaces.set(ws_id, name);
                herdr(&["workspace", "rename", ws_id, name]);
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
        cfg.set("space", "cwd");
        cfg.set("nope", "1");
        assert_eq!(
            cfg,
            Config {
                idle: Idle::Shell,
                agent: false,
                icons: true,
                max_len: 8,
                space: SpaceName::Cwd,
                icon_overrides: HashMap::new(),
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

    fn decide_tab(label: &str, number: Option<f64>, last_set: Option<&str>, name: &str) -> Action {
        decide(eligible(label, number, last_set), label, last_set, name)
    }

    #[test]
    fn decide_renames_records_or_backs_off() {
        let number = Some(2.0);
        // fresh tab still labelled with its number
        assert_eq!(decide_tab("2", number, None, "nvim"), Action::Rename);
        // label already ours but state was lost
        assert_eq!(decide_tab("nvim", number, None, "nvim"), Action::Skip);
        assert_eq!(decide_tab("nvim", number, Some("nvim"), "nvim"), Action::Skip);
        // our own tab, new program
        assert_eq!(
            decide_tab("nvim", number, Some("nvim"), "cargo"),
            Action::Rename
        );
        // herdr reset the label under us
        assert_eq!(decide_tab("2", number, Some("nvim"), "nvim"), Action::Rename);
        // user rename wins
        assert_eq!(
            decide_tab("logs", number, Some("nvim"), "cargo"),
            Action::Skip
        );
        assert_eq!(decide_tab("logs", number, None, "cargo"), Action::Skip);
    }

    #[test]
    fn decide_handles_tabs_without_a_number() {
        // a fresh tab herdr has not numbered yet
        assert_eq!(decide_tab("", None, None, "nvim"), Action::Rename);
        assert_eq!(decide_tab("nvim", None, Some("nvim"), "nvim"), Action::Skip);
        assert_eq!(decide_tab("logs", None, None, "nvim"), Action::Skip);
    }

    #[test]
    fn spaces_are_adopted_once_then_only_ours_to_rename() {
        let decide_space =
            |label: &str, last_set: Option<&str>, name: &str| -> Action {
                decide(space_eligible(label, last_set), label, last_set, name)
            };
        // herdr's own cwd-derived label, never touched by us
        assert_eq!(decide_space("cuentacero", None, ".dotfiles"), Action::Rename);
        assert_eq!(decide_space(".dotfiles", None, ".dotfiles"), Action::Record);
        // ours, and the active tab moved to another repo
        assert_eq!(
            decide_space(".dotfiles", Some(".dotfiles"), "sevastopol"),
            Action::Rename
        );
        assert_eq!(
            decide_space(".dotfiles", Some(".dotfiles"), ".dotfiles"),
            Action::Skip
        );
        // a label we did not set is the user's
        assert_eq!(decide_space("logs", Some(".dotfiles"), "api"), Action::Skip);
    }

    #[test]
    fn config_parses_file_with_comments() {
        let cfg = Config::parse(
            "# tab names\nidle = shell\nagent=no  # herdr detects it anyway\n\nmax_len = 12\nspace = off\nicons = off\nbogus\n",
        );
        assert_eq!(
            cfg,
            Config {
                idle: Idle::Shell,
                agent: false,
                icons: false,
                max_len: 12,
                space: SpaceName::Off,
                icon_overrides: HashMap::new(),
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
            spaces: Store::new(PathBuf::from("/nonexistent")),
            icons: Store::new(PathBuf::from("/nonexistent")),
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
        assert_eq!(Store::off_key("w1"), "w1.off");
    }

    #[test]
    fn off_markers_survive_gc_of_live_ids() {
        let dir = std::env::temp_dir().join(format!("herdr-autoname-off-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::new(dir.clone());

        store.set("wE", "dotfiles");
        store.set_off("wE");
        store.set_off("wZ");
        assert!(store.is_off("wE"));

        store.gc(&HashSet::from([Store::key("wE")]));
        assert!(store.is_off("wE"));
        assert_eq!(store.get("wE").as_deref(), Some("dotfiles"));
        assert!(!store.is_off("wZ"));

        store.clear_off("wE");
        assert!(!store.is_off("wE"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repo_root_climbs_to_the_git_directory() {
        let dir = std::env::temp_dir().join(format!("herdr-autoname-repo-{}", std::process::id()));
        let nested = dir.join("repo/crates/api");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(dir.join("repo/.git")).unwrap();

        assert_eq!(
            repo_root(nested.to_str().unwrap()),
            Some(dir.join("repo").to_path_buf())
        );
        assert_eq!(repo_root(dir.to_str().unwrap()), None);

        // a linked worktree carries a .git file instead of a directory
        let worktree = dir.join("wt");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(repo_root(worktree.to_str().unwrap()), Some(worktree));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn twins_are_numbered_from_the_second_one_on() {
        let bases = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(id, base)| ((*id).to_owned(), (*base).to_owned()))
                .collect::<Vec<_>>()
        };
        let names = |out: Vec<(String, String)>| {
            out.into_iter().map(|(_, name)| name).collect::<Vec<_>>()
        };

        // a name nobody shares never grows a number
        assert_eq!(
            names(number_twins(
                bases(&[("a", "dotfiles"), ("b", "nao")]),
                DEFAULT_MAX_LEN
            )),
            ["dotfiles", "nao"]
        );
        // twins are numbered in list order, and each name counts on its own
        assert_eq!(
            names(number_twins(
                bases(&[
                    ("a", "dotfiles"),
                    ("b", "nao"),
                    ("c", "dotfiles"),
                    ("d", "dotfiles"),
                ]),
                DEFAULT_MAX_LEN
            )),
            ["dotfiles", "nao", "dotfiles 2", "dotfiles 3"]
        );
        // the number lives inside the length budget, not past it
        assert_eq!(
            names(number_twins(
                bases(&[("a", "abcdef"), ("b", "abcdef")]),
                6
            )),
            ["abcdef", "abcd 2"]
        );
    }

    #[test]
    fn a_numbered_name_still_reads_as_its_own_base() {
        assert!(is_variant_of("dotfiles", "dotfiles"));
        assert!(is_variant_of("dotfiles 12", "dotfiles"));
        // a number is not a licence to match a different name
        assert!(!is_variant_of("dotfiles 2", "nao"));
        assert!(!is_variant_of("dotfiles beta", "dotfiles"));
        assert!(!is_variant_of("dotfiles ", "dotfiles"));
    }

    #[test]
    fn space_names_follow_the_repo_then_the_directory() {
        let dir = std::env::temp_dir().join(format!("herdr-autoname-space-{}", std::process::id()));
        let nested = dir.join("repo/crates/api");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(dir.join("repo/.git")).unwrap();
        let nested = nested.to_str().unwrap();

        let namer = |cfg| Namer {
            cfg,
            store: Store::new(PathBuf::from("/nonexistent")),
            spaces: Store::new(PathBuf::from("/nonexistent")),
            icons: Store::new(PathBuf::from("/nonexistent")),
        };

        assert_eq!(
            namer(Config::default()).name_from_cwd(nested).as_deref(),
            Some("repo")
        );
        // outside a repository the directory itself names the space
        let plain = dir.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        assert_eq!(
            namer(Config::default())
                .name_from_cwd(plain.to_str().unwrap())
                .as_deref(),
            Some("plain")
        );

        let cwd = namer(Config {
            space: SpaceName::Cwd,
            ..Config::default()
        });
        assert_eq!(cwd.name_from_cwd(nested).as_deref(), Some("api"));

        let off = namer(Config {
            space: SpaceName::Off,
            ..Config::default()
        });
        assert_eq!(off.name_from_cwd(nested), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn space_pane_prefers_focused_then_first_in_tab() {
        let panes: Vec<JsonValue> = [
            r#"{"pane_id": "p1", "tab_id": "t1", "focused": false}"#,
            r#"{"pane_id": "p2", "tab_id": "t2", "focused": false}"#,
            r#"{"pane_id": "p3", "tab_id": "t2", "focused": true}"#,
        ]
        .iter()
        .map(|pane| pane.parse().unwrap())
        .collect();
        let id = |pane: Option<&JsonValue>| {
            pane.and_then(|p| str_at(p, &["pane_id"]))
                .map(str::to_string)
        };
        // unlike a tab name, an unfocused multi-pane tab still names its space
        assert_eq!(id(space_pane(&panes, "t1")).as_deref(), Some("p1"));
        assert_eq!(id(space_pane(&panes, "t2")).as_deref(), Some("p3"));
        assert_eq!(id(space_pane(&panes, "t9")), None);
    }

    #[test]
    fn space_name_ignores_a_child_process_that_wandered_off() {
        let namer = Namer {
            cfg: Config::default(),
            store: Store::new(PathBuf::from("/nonexistent")),
            spaces: Store::new(PathBuf::from("/nonexistent")),
            icons: Store::new(PathBuf::from("/nonexistent")),
        };
        let pane: JsonValue = r#"{
            "pane_id": "wE:p1", "tab_id": "wE:t1",
            "cwd": "/nonexistent/dotfiles", "foreground_cwd": "/nonexistent/other"
        }"#
        .parse()
        .unwrap();
        assert_eq!(namer.name_for_space(&pane).as_deref(), Some("dotfiles"));
    }

    #[test]
    fn agent_icons_fall_back_and_bow_to_overrides() {
        let cfg = Config::default();
        assert_eq!(cfg.agent_icon("Claude Sonnet"), "\u{e9fb}");
        assert_eq!(cfg.agent_icon("codex"), "\u{e9fa}");
        assert_eq!(cfg.agent_icon("aider"), ICON_AGENT);

        let mut cfg = Config::default();
        cfg.set("icon.codex", "\u{f0e7}");
        cfg.set("icon.aider", "\u{f0e7}");
        assert_eq!(cfg.agent_icon("codex --sandbox"), "\u{f0e7}");
        assert_eq!(cfg.agent_icon("aider"), "\u{f0e7}");
        cfg.set("icon.codex", "");
        assert_eq!(cfg.agent_icon("codex"), "\u{e9fa}");
    }

    #[test]
    fn icon_overrides_are_trimmed_and_capped() {
        let mut cfg = Config::default();
        cfg.set("icon. GO ", "  go\u{7} ok  ");
        assert_eq!(cfg.icon_or("go", ICON_DIR), "go");
        cfg.set("icon.", "x");
        assert!(!cfg.icon_overrides.contains_key(""));
    }

    #[test]
    fn dir_icons_follow_the_repository_marker() {
        let dir = std::env::temp_dir().join(format!("herdr-autoname-icon-{}", std::process::id()));
        let nested = dir.join("repo/crates/api");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(dir.join("repo/.git")).unwrap();
        let nested = nested.to_str().unwrap();

        let mut cfg = Config::default();
        assert_eq!(cfg.dir_icon(nested), ICON_REPO);
        // the marker is looked for at the root, not beside the pane
        std::fs::write(dir.join("repo/Cargo.toml"), "").unwrap();
        assert_eq!(cfg.dir_icon(nested), "\u{e7a8}");
        // table order decides, not the filesystem: the language outranks the flake
        std::fs::write(dir.join("repo/flake.nix"), "").unwrap();
        assert_eq!(cfg.dir_icon(nested), "\u{e7a8}");
        cfg.set("icon.rust", "R");
        assert_eq!(cfg.dir_icon(nested), "R");

        let plain = dir.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        assert_eq!(
            Config::default().dir_icon(plain.to_str().unwrap()),
            ICON_DIR
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
