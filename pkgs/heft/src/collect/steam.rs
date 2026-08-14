use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::model::{Action, Bytes, DomainReport, Entry, Safety};
use crate::util::{days_since, expand_home, human};

/// Steam records an authoritative `SizeOnDisk` per app, so 200G+ is accounted
/// for by reading a handful of small text files instead of walking a tree.
pub struct SteamCollector {
    pub root: PathBuf,
    pub stale_days: i64,
}

impl Default for SteamCollector {
    fn default() -> Self {
        Self {
            root: PathBuf::from(expand_home("~/.local/share/Steam")),
            stale_days: 180,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub appid: String,
    pub name: String,
    pub size: Bytes,
    pub last_played: i64,
}

impl SteamCollector {
    pub fn collect(&self) -> Result<DomainReport> {
        let mut report = DomainReport::new("steam");

        if !self.root.is_dir() {
            report.notes.push("no Steam installation found".into());
            return Ok(report);
        }

        let libraries = self.libraries();
        let mut apps = Vec::new();
        for library in &libraries {
            apps.extend(read_library(library));
        }

        if apps.is_empty() {
            report.notes.push(format!(
                "found {} librar(ies) but no app manifests",
                libraries.len()
            ));
            return Ok(report);
        }

        apps.sort_by(|a, b| b.size.cmp(&a.size));
        report.bytes = apps.iter().map(|a| a.size).sum();
        report.notes.push(format!(
            "{} apps across {} librar(ies)",
            apps.len(),
            libraries.len()
        ));

        let mut stale_total = 0;
        for app in &apps {
            let idle = idle_days(app);
            let stale = idle.map_or(false, |d| d >= self.stale_days);
            if stale {
                stale_total += app.size;
            }

            let detail = match idle {
                Some(days) => format!("last played {days}d ago"),
                None => "never played".into(),
            };

            report.entries.push(
                Entry::new(app.name.clone(), app.size)
                    .detail(detail)
                    .last_used(Some(app.last_played).filter(|t| *t > 0))
                    .reclaimable(if stale { app.size } else { 0 }),
            );
        }

        if stale_total > 0 {
            let cold: Vec<&App> = apps
                .iter()
                .filter(|a| idle_days(a).map_or(false, |d| d >= self.stale_days))
                .collect();

            report.notes.push(format!(
                "{} apps untouched for {}+ days hold {}",
                cold.len(),
                self.stale_days,
                human(stale_total)
            ));

            if let Some(biggest) = cold.first() {
                report.actions.push(Action {
                    label: format!("uninstall \"{}\" (idle)", biggest.name),
                    frees: biggest.size,
                    command: format!("steam steam://uninstall/{}", biggest.appid),
                    safety: Safety::Review,
                });
            }
        }

        Ok(report)
    }

    /// Steam keeps additional libraries in `libraryfolders.vdf`; the install
    /// root is always one of them but is not always listed.
    fn libraries(&self) -> Vec<PathBuf> {
        let mut libraries = vec![self.root.join("steamapps")];

        let manifest = self.root.join("steamapps/libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(&manifest) {
            for (_, key, value) in parse_vdf(&text) {
                if key != "path" {
                    continue;
                }
                let candidate = Path::new(&value).join("steamapps");
                if candidate.is_dir() && !libraries.contains(&candidate) {
                    libraries.push(candidate);
                }
            }
        }

        libraries.retain(|p| p.is_dir());
        libraries
    }
}

fn idle_days(app: &App) -> Option<i64> {
    (app.last_played > 0).then(|| days_since(app.last_played))
}

fn read_library(library: &Path) -> Vec<App> {
    let Ok(entries) = fs::read_dir(library) else {
        return Vec::new();
    };

    let mut apps = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
            continue;
        }
        let Ok(text) = fs::read_to_string(entry.path()) else {
            continue;
        };
        if let Some(app) = parse_manifest(&text) {
            apps.push(app);
        }
    }
    apps
}

pub fn parse_manifest(text: &str) -> Option<App> {
    let fields: HashMap<String, String> = parse_vdf(text)
        .into_iter()
        .filter(|(depth, _, _)| *depth == 1)
        .map(|(_, key, value)| (key, value))
        .collect();

    let size = fields.get("SizeOnDisk")?.parse().ok()?;
    Some(App {
        appid: fields.get("appid").cloned().unwrap_or_default(),
        name: fields
            .get("name")
            .cloned()
            .unwrap_or_else(|| "unknown".into()),
        size,
        last_played: fields
            .get("LastPlayed")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    })
}

/// Minimal VDF reader: yields `(depth, key, value)` for every scalar pair.
/// Depth 1 is the body of the single top-level object.
fn parse_vdf(text: &str) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut pending: Option<String> = None;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                depth += 1;
                pending = None;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                pending = None;
            }
            '"' => {
                let mut token = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                token.push(escaped);
                            }
                        }
                        '"' => break,
                        _ => token.push(c),
                    }
                }
                match pending.take() {
                    Some(key) => out.push((depth, key, token)),
                    None => pending = Some(token),
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
"AppState"
{
	"appid"		"1091500"
	"universe"		"1"
	"name"		"Cyberpunk 2077"
	"StateFlags"		"4"
	"installdir"		"Cyberpunk 2077"
	"LastUpdated"		"1775961361"
	"LastPlayed"		"1785941264"
	"SizeOnDisk"		"91328398342"
	"InstalledDepots"
	{
		"1091501"
		{
			"manifest"		"4956520704156364317"
			"size"		"222685392"
		}
	}
}
"#;

    #[test]
    fn manifest_yields_name_size_and_last_played() {
        let app = parse_manifest(MANIFEST).expect("manifest parses");
        assert_eq!(app.appid, "1091500");
        assert_eq!(app.name, "Cyberpunk 2077");
        assert_eq!(app.size, 91_328_398_342);
        assert_eq!(app.last_played, 1_785_941_264);
    }

    #[test]
    fn nested_blocks_do_not_leak_into_top_level_fields() {
        // "size" inside InstalledDepots must not be mistaken for SizeOnDisk,
        // and "manifest" must not surface at depth 1 at all.
        let pairs = parse_vdf(MANIFEST);
        let depth_one: Vec<&str> = pairs
            .iter()
            .filter(|(d, _, _)| *d == 1)
            .map(|(_, k, _)| k.as_str())
            .collect();
        assert!(depth_one.contains(&"SizeOnDisk"));
        assert!(!depth_one.contains(&"manifest"));
        assert!(!depth_one.contains(&"size"));
    }

    #[test]
    fn manifest_without_size_is_rejected() {
        assert!(parse_manifest("\"AppState\"\n{\n\t\"appid\" \"1\"\n}\n").is_none());
    }

    #[test]
    fn library_paths_are_collected_at_any_depth() {
        let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/home/u/.local/share/Steam"
	}
	"1"
	{
		"path"		"/mnt/games/SteamLibrary"
	}
}
"#;
        let paths: Vec<String> = parse_vdf(vdf)
            .into_iter()
            .filter(|(_, k, _)| k == "path")
            .map(|(_, _, v)| v)
            .collect();
        assert_eq!(paths, vec!["/home/u/.local/share/Steam", "/mnt/games/SteamLibrary"]);
    }
}
