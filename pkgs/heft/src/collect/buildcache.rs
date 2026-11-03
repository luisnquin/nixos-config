use std::path::PathBuf;

use anyhow::Result;

use crate::model::{Action, DomainReport, Entry, Safety};
use crate::util::{collapse_home, days_since, expand_home, human, Walker};

/// Artifact directories are regenerable by definition, so their value is not
/// total size but how much of it has gone cold.
pub struct BuildCacheCollector {
    pub paths: Vec<String>,
    pub cold_days: i64,
}

/// Chosen from an actual census of this machine: the shared Rust target dir
/// alone held 45G, and the Android SDK another 32G.
pub const DEFAULT_PATHS: &[&str] = &[
    "~/.local/share/target",
    "~/.cache",
    "~/.gradle",
    "~/.android/avd",
    "~/.android/ndk",
    "~/.npm",
    "~/.npm-global",
    "~/.bun-global",
    "~/.rustup",
    "~/.cargo",
    "~/.local/share/.cargo",
    "~/.local/share/pnpm",
];

const PURE_CACHES: &[&str] = &["~/.cache", "~/.local/share/target", "~/.npm"];

impl Default for BuildCacheCollector {
    fn default() -> Self {
        Self {
            paths: DEFAULT_PATHS.iter().map(|s| s.to_string()).collect(),
            cold_days: 90,
        }
    }
}

impl BuildCacheCollector {
    pub fn expanded(&self) -> Vec<PathBuf> {
        self.paths
            .iter()
            .map(|p| PathBuf::from(expand_home(p)))
            .collect()
    }

    pub fn collect(&self) -> Result<DomainReport> {
        let mut report = DomainReport::new("build-caches");
        let mut cold_total = 0;
        let mut coldest: Option<(String, u64, bool)> = None;

        for path in self.expanded() {
            if !path.exists() {
                continue;
            }

            // A fresh walker per root: hardlinks are shared *within* a cache
            // far more often than between two unrelated caches, and sharing one
            // walker would bill the overlap to whichever root ran first.
            let mut walker = Walker::new(self.cold_days);
            let result = walker.walk(&path);
            if result.bytes == 0 {
                continue;
            }

            report.bytes += result.bytes;
            cold_total += result.cold;

            let label = collapse_home(&path);
            let detail = if result.newest > 0 {
                format!(
                    "{} files · {} cold · touched {}d ago",
                    result.files,
                    human(result.cold),
                    days_since(result.newest)
                )
            } else {
                format!("{} files", result.files)
            };

            // A tree that is cold end to end can be removed outright; a partly
            // cold one must be pruned file by file or the live half goes too.
            let entirely_cold = result.cold == result.bytes;
            if PURE_CACHES.contains(&label.as_str()) {
                if result.cold > 0 {
                    report.actions.push(self.cold_action(
                        &label,
                        result.cold,
                        entirely_cold,
                        Safety::Safe,
                    ));
                }
            } else if result.cold > 0 && coldest.as_ref().map_or(true, |(_, b, _)| result.cold > *b)
            {
                coldest = Some((label.clone(), result.cold, entirely_cold));
            }

            report.entries.push(
                Entry::new(label, result.bytes)
                    .detail(detail)
                    .last_used(Some(result.newest).filter(|t| *t > 0))
                    .reclaimable(result.cold),
            );
        }

        if cold_total > 0 {
            report.notes.push(format!(
                "{} has not been touched in {}+ days",
                human(cold_total),
                self.cold_days
            ));
        }

        if let Some((label, bytes, entirely_cold)) = coldest {
            report
                .actions
                .push(self.cold_action(&label, bytes, entirely_cold, Safety::Review));
        }

        report.truncate_entries(12);
        Ok(report)
    }

    fn cold_action(&self, label: &str, bytes: u64, entirely_cold: bool, safety: Safety) -> Action {
        Action {
            label: if entirely_cold {
                format!("remove {label}, cold throughout")
            } else {
                format!("clear cold artifacts in {label}")
            },
            frees: bytes,
            command: if entirely_cold {
                format!("rm -rf {label}")
            } else {
                // mtime, not atime: relatime makes access times unreliable,
                // and the cold split is measured on mtime too.
                format!("find {label} -type f -mtime +{} -delete", self.cold_days)
            },
            safety,
        }
    }
}
