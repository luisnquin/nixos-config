mod center;
mod config;
mod format;
mod mako;
mod state;
mod watch;

use config::Config;
use mako::DND_MODE;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: hark <waybar|centre|seen|dnd|drop|restore|invoke|clear [group]> [options]";

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(command) = arguments.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };

    match run(&command, Options::parse(arguments)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hark: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Default)]
struct Options {
    watch: bool,
    id: Option<u32>,
    action: Option<String>,
    config: Option<PathBuf>,
    rest: Vec<String>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Options {
        let mut options = Options::default();
        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--watch" => options.watch = true,
                "--id" | "-n" => options.id = arguments.next().and_then(|id| id.parse().ok()),
                "--action" => options.action = arguments.next(),
                "--config" => options.config = arguments.next().map(PathBuf::from),
                _ => options.rest.push(argument),
            }
        }
        options
    }

    fn id(&self) -> Result<u32, Box<dyn Error>> {
        self.id.ok_or_else(|| "--id is required".into())
    }
}

fn run(command: &str, options: Options) -> Result<(), Box<dyn Error>> {
    match command {
        "waybar" => render(options, |centre, config| centre.waybar(config)),
        "centre" | "center" => render(options, |centre, config| centre.model(config)),
        "seen" => state::mark_seen(),
        "dnd" => dnd(options.rest.first().map(String::as_str).unwrap_or("toggle")),
        "clear" => match options.rest.first() {
            Some(group) => clear_group(group, options.config),
            None => clear(),
        },
        "drop" => drop_notification(options.id()?),
        "restore" => mako::control(&["restore", "-n", &options.id()?.to_string()]).map(|_| ()),
        "invoke" => invoke(options.id()?, options.action.as_deref()),
        _ => Err(format!("unknown command {command}\n{USAGE}").into()),
    }
}

fn render(
    options: Options,
    shape: impl Fn(&center::Centre, &Config) -> Value,
) -> Result<(), Box<dyn Error>> {
    let config = Config::load(options.config)?;
    let emit = || {
        let centre = center::build(&config, state::seen_at());
        println!("{}", shape(&centre, &config));
        let _ = std::io::stdout().flush();
    };

    if options.watch {
        watch::run(emit);
    }
    emit();
    Ok(())
}

fn dnd(mode: &str) -> Result<(), Box<dyn Error>> {
    let flag = match mode {
        "on" => "-a",
        "off" => "-r",
        "toggle" => "-t",
        "status" => {
            let enabled = mako::modes().iter().any(|mode| mode == DND_MODE);
            println!("{enabled}");
            return Ok(());
        }
        other => return Err(format!("unknown dnd mode {other}").into()),
    };
    mako::control(&["mode", flag, DND_MODE]).map(|_| ())
}

fn clear() -> Result<(), Box<dyn Error>> {
    // Dismissing without history first, so the sweep does not refill the ring
    // it is about to empty.
    let _ = mako::control(&["dismiss", "-a", "-h"]);
    mako::control(&["history-clear"]).map(|_| ())
}

fn clear_group(key: &str, config: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let config = Config::load(config)?;
    let centre = center::build(&config, state::seen_at());
    let group = centre
        .groups
        .iter()
        .find(|group| group.key == key)
        .ok_or_else(|| format!("no group {key}"))?;

    // The centre already knows which list each entry sits in, so the sweep
    // does not re-ask mako once per notification.
    for entry in &group.entries {
        drop_one(entry.id, entry.active)?;
    }
    Ok(())
}

fn drop_notification(id: u32) -> Result<(), Box<dyn Error>> {
    let active = mako::collect()
        .unwrap_or_default()
        .iter()
        .any(|notification| notification.active && notification.id == id);
    drop_one(id, active)
}

fn drop_one(id: u32, active: bool) -> Result<(), Box<dyn Error>> {
    let id = id.to_string();
    if active {
        return mako::control(&["dismiss", "-n", &id, "-h"]).map(|_| ());
    }
    mako::control(&["history-remove", "-n", &id]).map(|_| ())
}

fn invoke(id: u32, action: Option<&str>) -> Result<(), Box<dyn Error>> {
    let id = id.to_string();
    let notifications = mako::collect()?;
    let notification = notifications
        .iter()
        .find(|notification| notification.id.to_string() == id)
        .ok_or_else(|| format!("no notification {id}"))?;

    let key = match action {
        Some(key) => key.to_owned(),
        None => notification
            .actions
            .iter()
            .find(|action| action.key == "default")
            .or_else(|| notification.actions.first())
            .map(|action| action.key.clone())
            .ok_or("notification has no actions")?,
    };

    // A history notification needs -H, and mako drops it from the ring once the
    // action fires; an on-screen one is invoked in place.
    if notification.active {
        mako::control(&["invoke", "-n", &id, &key]).map(|_| ())
    } else {
        mako::control(&["invoke", "-H", "-n", &id, &key]).map(|_| ())
    }
}


/// A key safe to embed in a CSS class, a shell word and an eww regex.
pub fn slug(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut pending_dash = false;
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if slug.is_empty() {
        "unknown".to_owned()
    } else {
        slug
    }
}
