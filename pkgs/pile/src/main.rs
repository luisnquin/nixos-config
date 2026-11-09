#![forbid(unsafe_code)]

mod format;
mod mako;

use serde_json::{json, Value};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const USAGE: &str = "usage: pile <push [--keep] <id>|waybar|panel|drop <key>|clear>";

const PKILL: &str = "@pkill@";
const SIGNAL: &str = "@signal@";

const KEEP: usize = 50;
const ROW_CHARS: usize = 42;
const ROW_ALPHA: [&str; 3] = ["100%", "70%", "45%"];
const FLASH_MS: u64 = 2000;
const FLASH_FRAMES: u32 = 10;

struct Entry {
    id: u32,
    created_at: u64,
    app: String,
    summary: String,
    body: String,
    urgency: String,
    image: String,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["push", id] | ["push", "--keep", id] => id
            .parse()
            .map_err(|_| format!("bad id {id}").into())
            .and_then(|id| push(id, arguments[1] == "--keep")),
        ["waybar"] => {
            println!("{}", waybar(&load(), now_ms()));
            Ok(())
        }
        ["panel"] => {
            println!("{}", panel(&load()));
            Ok(())
        }
        ["drop", key] => {
            let mut entries = load();
            entries.retain(|entry| self::key(entry) != key);
            save(&entries).map(|()| refresh())
        }
        ["clear"] => save(&[]).map(|()| refresh()),
        _ => Err(USAGE.into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pile: {error}");
            ExitCode::FAILURE
        }
    }
}

fn push(id: u32, keep: bool) -> Result<(), Box<dyn Error>> {
    let notification = mako::active(id)?;
    let mut entries = load();

    entries.retain(|entry| !(entry.id == id && entry.app == notification.app_name));
    entries.insert(
        0,
        Entry {
            id,
            created_at: notification.created_at,
            app: notification.app_name,
            summary: notification.summary,
            body: notification.body,
            urgency: notification.urgency,
            image: attachment(notification.app_icon),
        },
    );
    entries.truncate(KEEP);
    save(&entries)?;

    if !keep {
        mako::control(&["dismiss", "-n", &id.to_string(), "--no-history"])?;
    }
    refresh();
    for _ in 0..FLASH_FRAMES {
        std::thread::sleep(Duration::from_millis(FLASH_MS) / FLASH_FRAMES);
        refresh();
    }
    Ok(())
}

fn attachment(app_icon: String) -> String {
    if app_icon.starts_with('/') && !app_icon.starts_with("/nix/store/") { app_icon } else { String::new() }
}

fn line(entry: &Entry) -> String {
    let mut parts = vec![entry.app.trim_start_matches('.').trim_end_matches("-wrapped")];
    for part in entry.summary.split('·').map(str::trim) {
        if !part.is_empty() && !parts.iter().any(|kept| kept.eq_ignore_ascii_case(part)) {
            parts.push(part);
        }
    }
    parts.join(" · ")
}

fn waybar(entries: &[Entry], now: u64) -> Value {
    let row = |entry: &Entry| format::escape_markup(&format::truncate(&line(entry), ROW_CHARS));

    let text = entries
        .iter()
        .zip(ROW_ALPHA)
        .map(|(entry, alpha)| {
            let age = now.saturating_sub(entry.created_at);
            let colour = if age < FLASH_MS {
                format!(" foreground=\"#ffffff\" background=\"#ff5555\" bgalpha=\"{}%\"", 1 + 70 * (FLASH_MS - age) / FLASH_MS)
            } else if entry.urgency == "critical" {
                " foreground=\"#ff5555\"".to_owned()
            } else {
                String::new()
            };
            format!("<span alpha=\"{alpha}\"{colour}>{}  {}</span>", format::clock(entry.created_at), row(entry))
        })

        .collect::<Vec<_>>()
        .join("\n");

    json!({ "text": text, "class": if entries.is_empty() { "empty" } else { "filled" } })
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

fn key(entry: &Entry) -> String {
    format!("{}-{}", entry.id, entry.created_at)
}

fn panel(entries: &[Entry]) -> Value {
    let rows: Vec<Value> = entries
        .iter()
        .map(|entry| {
            json!({
                "key": key(entry),
                "line": line(entry),
                "body": entry.body.trim(),
                "clock": format::clock(entry.created_at),
                "urgency": entry.urgency,
                "images": std::iter::once(&entry.image).filter(|image| Path::new(image).is_file()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let height = (entries.len() * 22).clamp(22, 420);
    json!({ "empty": entries.is_empty(), "count": entries.len(), "height": height, "entries": rows })
}

fn path() -> PathBuf {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    state.join("pile/entries.json")
}

fn load() -> Vec<Entry> {
    let Ok(raw) = std::fs::read_to_string(path()) else {
        return Vec::new();
    };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&raw) else {
        return Vec::new();
    };
    let text = |entry: &Value, key: &str| entry[key].as_str().unwrap_or_default().to_owned();
    entries
        .iter()
        .map(|entry| Entry {
            id: entry["id"].as_u64().unwrap_or(0) as u32,
            created_at: entry["created_at"].as_u64().unwrap_or(0),
            app: text(entry, "app"),
            summary: text(entry, "summary"),
            body: text(entry, "body"),
            urgency: text(entry, "urgency"),
            image: text(entry, "image"),
        })
        .collect()
}

fn save(entries: &[Entry]) -> Result<(), Box<dyn Error>> {
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let value: Vec<Value> = entries
        .iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "created_at": entry.created_at,
                "app": entry.app,
                "summary": entry.summary,
                "body": entry.body,
                "urgency": entry.urgency,
                "image": entry.image,
            })
        })
        .collect();
    let staging = path.with_extension("json.tmp");
    std::fs::write(&staging, serde_json::to_string(&value)?)?;
    std::fs::rename(staging, path)?;
    Ok(())
}

/// Under nix the bar runs as .waybar-wrapped, so both names are signalled.
fn refresh() {
    let _ = Command::new(PKILL)
        .args([&format!("-RTMIN+{SIGNAL}"), "-x", "waybar|\\.waybar-wrapped"])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(app: &str, summary: &str, urgency: &str) -> Entry {
        Entry {
            id: 1,
            created_at: 0,
            app: app.into(),
            summary: summary.into(),
            body: String::new(),
            urgency: urgency.into(),
            image: String::new(),
        }
    }

    #[test]
    fn only_off_store_paths_are_attachments() {
        assert_eq!(attachment("/tmp/shot.png".into()), "/tmp/shot.png");
        assert_eq!(attachment("/nix/store/x-sys-sound/volume.png".into()), "");
        assert_eq!(attachment("dialog-information".into()), "");
        let mut gone = entry("Hyprshot", "Screenshot saved", "normal");
        gone.image = "/nonexistent/shot.png".into();
        assert_eq!(panel(&[gone])["entries"][0]["images"], json!([]));
    }

    #[test]
    fn bar_shows_three_newest_fading_and_escaped() {
        let entries = [
            entry("phone", "lock <pixel>", "critical"),
            entry("grimblast", "Screenshot", "normal"),
            entry("Herdr", "claude finished", "normal"),
            entry("Claude Code", "dropped", "normal"),
        ];
        let rendered = waybar(&entries, FLASH_MS);
        let deduped = waybar(&[entry("Claude Code", "Claude Code · sevastopol", "normal")], FLASH_MS);
        assert!(deduped["text"].as_str().unwrap().ends_with(" Claude Code · sevastopol</span>"));
        let text = rendered["text"].as_str().unwrap();
        let rows: Vec<&str> = text.lines().collect();

        assert_eq!(rows.len(), 3);
        assert!(rows[0].starts_with("<span alpha=\"100%\" foreground=\"#ff5555\">  phone · lock &lt;pixel&gt;"));
        assert!(rows[1].starts_with("<span alpha=\"70%\">  grimblast"));
        assert!(rows[2].starts_with("<span alpha=\"45%\">  Herdr"));
        assert!(rendered.get("tooltip").is_none());
        assert_eq!(waybar(&[], 0)["class"], "empty");

        let fresh = waybar(&[entry("phone", "lock", "critical")], FLASH_MS / 2);
        assert!(fresh["text"].as_str().unwrap().starts_with("<span alpha=\"100%\" foreground=\"#ffffff\" background=\"#ff5555\" bgalpha=\"36%\">"));
    }
}
