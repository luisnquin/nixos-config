use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{self, Unix, View};
use crate::project::Project;
use crate::{adb, agent, apps, registry};

const TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    pub app: String,
    pub pid: Option<String>,
    /// `lastUpdateTime` as the package manager printed it; `None` once it
    /// printed nothing, which is an app no longer installed.
    pub updated: Option<String>,
    pub at: Unix,
}

pub struct Look {
    path: PathBuf,
    device: String,
    seen: BTreeMap<String, Seen>,
    now: Seen,
    said: Option<String>,
}

impl Look {
    pub fn settle(mut self) {
        if let Some(said) = &self.said {
            eprintln!("phone: {said}");
        }

        self.seen.insert(self.device, self.now);

        if let Err(e) = save(&self.path, &self.seen) {
            eprintln!("phone: could not remember the app's pid: {e:#}");
        }
    }
}

pub async fn look(view: &View) -> Option<Look> {
    if !view.device.platform.is_adb() {
        return None;
    }

    let serial = view.reach.serial()?;
    let path = agent::file("pids", &agent::me().id);
    let project = Project::here().ok().flatten()?;
    let app = apps::app_id(&project.manifest.build.get("android")?.app)
        .ok()?
        .to_string();

    let mut seen = load(&path);
    let before = seen
        .remove(&view.device.id)
        .filter(|s| s.app == app && s.pid.as_deref().is_none_or(is_pid));

    let last = match &before {
        Some(s) => s.pid.clone().unwrap_or_default(),
        None => "-".to_string(),
    };

    let script = format!(
        r#"set -- $(pidof {app} 2>/dev/null)
echo "$1"
[ "$1" = "{last}" ] && exit 0
dumpsys package {app} | grep -m1 lastUpdateTime"#
    );

    let out = adb::run_timeout(&view.server, &["-s", serial, "shell", &script], TIMEOUT)
        .await
        .ok()?;

    let text = out.stdout;
    let mut lines = text.lines();
    let pid = lines
        .next()
        .map(str::trim)
        .filter(|p| is_pid(p))
        .map(str::to_string);
    let asked = pid.as_deref().unwrap_or_default() != last;
    let updated = match asked {
        true => lines.find_map(|l| l.trim().strip_prefix("lastUpdateTime=")),
        false => None,
    }
    .map(|u| u.trim().to_string());

    let now = Seen {
        app: app.clone(),
        pid,
        updated: match asked {
            true => updated,
            false => before.as_ref().and_then(|s| s.updated.clone()),
        },
        at: model::now(),
    };

    let said = before
        .as_ref()
        .and_then(|b| change(&view.device.label, b, &now));

    Some(Look {
        path,
        device: view.device.id.clone(),
        seen,
        now,
        said,
    })
}

pub fn forget(device: &str) {
    let path = agent::file("pids", &agent::me().id);
    let mut seen = load(&path);

    if seen.remove(device).is_some() {
        let _ = save(&path, &seen);
    }
}

fn is_pid(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

pub fn change(label: &str, before: &Seen, now: &Seen) -> Option<String> {
    let app = &now.app;
    let since = format!("since this session's last verb ({})", model::ago(before.at));

    let reinstalled = match (&before.updated, &now.updated) {
        (Some(was), Some(is)) if was != is => Some(is.as_str()),
        _ => None,
    };
    let gone = before.updated.is_some() && now.updated.is_none() && now.pid.is_none();

    if gone {
        return Some(format!("{app} was uninstalled from {label} {since}"));
    }

    let why = match reinstalled {
        Some(at) => format!("lastUpdateTime {at}: reinstalled"),
        None => "lastUpdateTime unchanged, so not reinstalled: it crashed, or something stopped it"
            .to_string(),
    };

    match (&before.pid, &now.pid) {
        (Some(was), Some(is)) if was != is => Some(format!(
            "{app} restarted on {label} {since}: pid {was}, now {is}; {why}"
        )),
        (Some(was), None) => Some(format!(
            "{app} is no longer running on {label} {since} (was pid {was}); {why}"
        )),
        _ => reinstalled
            .map(|at| format!("{app} was reinstalled on {label} {since}: lastUpdateTime {at}")),
    }
}

fn load(path: &Path) -> BTreeMap<String, Seen> {
    std::fs::read(path)
        .ok()
        .and_then(|body| serde_json::from_slice(&body).ok())
        .unwrap_or_default()
}

fn save(path: &Path, seen: &BTreeMap<String, Seen>) -> Result<()> {
    let dir = path.parent().expect("a session file has a directory");

    std::fs::create_dir_all(dir)?;
    let _ = std::fs::remove_dir_all(registry::state_dir().join("sessions"));

    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));

    std::fs::write(&tmp, serde_json::to_vec_pretty(seen)?)?;
    std::fs::rename(&tmp, path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(pid: Option<&str>, updated: Option<&str>) -> Seen {
        Seen {
            app: "dev.luisnquin.dazzle".to_string(),
            pid: pid.map(str::to_string),
            updated: updated.map(str::to_string),
            at: model::now(),
        }
    }

    const T0: &str = "2026-09-25 06:10:00";
    const T1: &str = "2026-09-25 06:23:07";

    #[test]
    fn the_same_process_says_nothing() {
        let was = seen(Some("100"), Some(T0));

        assert_eq!(change("pixel", &was, &was.clone()), None);
    }

    #[test]
    fn a_new_pid_with_a_new_update_time_is_a_reinstall() {
        let said = change(
            "pixel",
            &seen(Some("100"), Some(T0)),
            &seen(Some("200"), Some(T1)),
        )
        .unwrap();

        assert!(said
            .starts_with("dev.luisnquin.dazzle restarted on pixel since this session's last verb"));
        assert!(said.contains("pid 100, now 200"));
        assert!(said.ends_with("lastUpdateTime 2026-09-25 06:23:07: reinstalled"));
    }

    #[test]
    fn a_new_pid_alone_is_a_restart_and_not_a_reinstall() {
        let said = change(
            "pixel",
            &seen(Some("100"), Some(T0)),
            &seen(Some("200"), Some(T0)),
        )
        .unwrap();

        assert!(said.contains("not reinstalled"));
    }

    #[test]
    fn a_process_that_went_away_is_named() {
        let said = change("pixel", &seen(Some("100"), Some(T0)), &seen(None, Some(T1))).unwrap();

        assert!(said.contains("is no longer running on pixel"));
        assert!(said.contains("(was pid 100)"));
        assert!(said.ends_with("reinstalled"));
    }

    #[test]
    fn a_package_with_no_update_time_left_was_uninstalled() {
        let said = change("pixel", &seen(Some("100"), Some(T0)), &seen(None, None)).unwrap();

        assert!(said.contains("was uninstalled from pixel"));
    }

    #[test]
    fn an_app_that_starts_is_not_news_unless_it_was_reinstalled() {
        assert_eq!(
            change("pixel", &seen(None, Some(T0)), &seen(Some("200"), Some(T0))),
            None
        );
        assert!(
            change("pixel", &seen(None, Some(T0)), &seen(Some("200"), Some(T1)))
                .unwrap()
                .contains("was reinstalled on pixel")
        );
    }

    #[test]
    fn an_agent_id_is_hashed_into_a_file_name() {
        let plain = agent::file("pids", "1f636eb3-8a91/a6a8204d");
        let odd = agent::file("pids", "../../etc/passwd");

        assert!(plain.parent().unwrap().ends_with("pids"));
        assert_eq!(odd.parent(), plain.parent());
        assert!(!odd.to_string_lossy().contains(".."));
    }
}
