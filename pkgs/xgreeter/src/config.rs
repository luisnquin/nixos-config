use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;

use crate::theme::{parse_hex, Accent, Overrides};

/// 0xc000022070's greeter - a ctOS-flavored ratatui frontend for greetd.
#[derive(Debug, Parser)]
#[command(name = "xgreeter", version, about)]
pub struct Cli {
    /// Path to a TOML config file. CLI flags override its values.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Run without a real greetd socket, using an in-process mock. Safe to run
    /// in any terminal: it never authenticates or launches a session.
    #[arg(long)]
    pub demo: bool,

    /// Prefill the username field.
    #[arg(long)]
    pub user: Option<String>,

    /// Ignore login entirely and run the ascii animation full-screen, looping.
    /// A hands-off showcase — no greetd, no auth. ESC/q quits.
    #[arg(long)]
    pub ascii_demo: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// argv greetd execs as the session, not a shell string.
    pub session_cmd: Vec<String>,
    pub default_user: String,
    pub idle_status: String,
    pub log_cmd: Vec<String>,
    pub accent: Accent,
    pub overrides: Overrides,
    pub disclaimer: Option<String>,
    pub show_help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            session_cmd: vec!["start-hyprland".into()],
            default_user: String::new(),
            idle_status: "AWAITING IDENTIFICATION".into(),
            log_cmd: shell_words("journalctl -b -n 40 -f -o cat"),
            accent: Accent::Amber,
            overrides: Overrides::default(),
            disclaimer: None,
            show_help: true,
        }
    }
}

// Every field optional so a partial file layers over defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    session_cmd: Option<Vec<String>>,
    default_user: Option<String>,
    idle_status: Option<String>,
    log_cmd: Option<Vec<String>>,
    accent: Option<Accent>,
    show_help: Option<bool>,
    disclaimer: Option<String>,
    disclaimer_path: Option<PathBuf>,
    #[serde(default)]
    colors: ColorsFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ColorsFile {
    accent: Option<String>,
    on_accent: Option<String>,
    foreground: Option<String>,
    dim: Option<String>,
    error: Option<String>,
    background: Option<String>,
    field_background: Option<String>,
}

impl Config {
    pub fn load(cli: &Cli) -> Result<Config> {
        let mut cfg = Config::default();

        // Explicit --config wins; otherwise the first well-known path that
        // exists, so an unconfigured greetd unit can exec the bare binary.
        // Neither present: the built-in defaults stand.
        if let Some(path) = cli.config.clone().or_else(default_config_path) {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading config {}", path.display()))?;
            let file: FileConfig = toml::from_str(&raw)
                .with_context(|| format!("parsing config {}", path.display()))?;
            cfg.apply_file(file, &path)?;
        }

        if let Some(u) = &cli.user {
            cfg.default_user = u.clone();
        }

        Ok(cfg)
    }

    fn apply_file(&mut self, file: FileConfig, cfg_path: &std::path::Path) -> Result<()> {
        if let Some(v) = file.session_cmd {
            self.session_cmd = v;
        }
        if let Some(v) = file.default_user {
            self.default_user = v;
        }
        if let Some(v) = file.idle_status {
            self.idle_status = v;
        }
        if let Some(v) = file.log_cmd {
            self.log_cmd = v;
        }
        if let Some(v) = file.accent {
            self.accent = v;
        }
        if let Some(v) = file.show_help {
            self.show_help = v;
        }

        // disclaimer_path (resolved relative to the config file) wins over the
        // inline string.
        self.disclaimer = load_text(
            file.disclaimer,
            file.disclaimer_path,
            cfg_path,
            "disclaimer",
        )?;

        self.overrides = parse_overrides(&file.colors)?;
        Ok(())
    }
}

fn load_text(
    inline: Option<String>,
    path: Option<PathBuf>,
    cfg_path: &std::path::Path,
    label: &str,
) -> Result<Option<String>> {
    if let Some(p) = path {
        let resolved = if p.is_absolute() {
            p
        } else {
            cfg_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(p)
        };
        let text = std::fs::read_to_string(&resolved)
            .with_context(|| format!("reading {label} file {}", resolved.display()))?;
        return Ok(Some(text));
    }
    Ok(inline)
}

fn parse_overrides(c: &ColorsFile) -> Result<Overrides> {
    let f = |s: &Option<String>| -> Result<Option<ratatui::style::Color>> {
        match s {
            Some(v) => Ok(Some(parse_hex(v).map_err(anyhow::Error::msg)?)),
            None => Ok(None),
        }
    };
    Ok(Overrides {
        accent: f(&c.accent)?,
        on_accent: f(&c.on_accent)?,
        fg: f(&c.foreground)?,
        dim: f(&c.dim)?,
        error: f(&c.error)?,
        bg: f(&c.background)?,
        field_bg: f(&c.field_background)?,
    })
}

/// Well-known config locations, checked in order. Returns the first that
/// exists so an unconfigured greetd unit can exec the bare binary: a per-user
/// file for terminal/`--demo` use, then the system path the NixOS module writes.
fn default_config_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        candidates.push(PathBuf::from(dir).join("xgreeter/config.toml"));
    } else if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".config/xgreeter/config.toml"));
    }
    candidates.push(PathBuf::from("/etc/xgreeter/config.toml"));
    candidates.into_iter().find(|p| p.is_file())
}

/// Minimal whitespace splitter for the built-in default only. Config files
/// should pass real argv arrays for anything with quoting.
fn shell_words(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}
