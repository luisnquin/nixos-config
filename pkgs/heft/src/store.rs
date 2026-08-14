use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::{DomainSlice, Mover, RankedAction, Rollup, Snapshot};
use crate::util::human_delta;

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HEFT_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.local/share")
    });
    PathBuf::from(base).join("heft")
}

fn snapshot_dir() -> PathBuf {
    data_dir().join("snapshots")
}

pub fn rollup_path() -> PathBuf {
    data_dir().join("latest.json")
}

/// Write through a temporary file in the same directory, then rename. A reader
/// polling this path either sees the previous complete file or the new one,
/// never a half-written document.
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("heft")
    ));
    fs::write(&tmp, contents).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("renaming into {}", path.display()))?;
    Ok(())
}

pub fn save_snapshot(snapshot: &Snapshot) -> Result<PathBuf> {
    let name = snapshot.taken_at.format("%Y-%m-%d.json").to_string();
    let path = snapshot_dir().join(name);
    write_atomic(&path, &serde_json::to_string_pretty(snapshot)?)?;
    Ok(path)
}

pub fn save_rollup(rollup: &Rollup) -> Result<PathBuf> {
    let path = rollup_path();
    write_atomic(&path, &serde_json::to_string(rollup)?)?;
    Ok(path)
}

pub fn read_rollup() -> Result<Rollup> {
    let path = rollup_path();
    let text = fs::read_to_string(&path)
        .with_context(|| format!("reading {} — run `heft scan` first", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

/// Snapshot files sorted oldest first. The name is the date, so lexical order
/// is chronological order.
pub fn snapshot_paths() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(snapshot_dir()) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
}

pub fn load_snapshot(path: &Path) -> Result<Snapshot> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

pub fn latest_snapshot() -> Option<Snapshot> {
    snapshot_paths()
        .last()
        .and_then(|p| load_snapshot(p).ok())
}

/// The snapshot before the newest one, which is what a day-over-day delta
/// compares against.
pub fn previous_snapshot() -> Option<Snapshot> {
    let paths = snapshot_paths();
    if paths.len() < 2 {
        return None;
    }
    load_snapshot(&paths[paths.len() - 2]).ok()
}

/// Resolve a user-supplied snapshot reference: a date (`2026-08-13`), a
/// negative offset from newest (`-1`), or a path.
pub fn resolve(reference: &str) -> Result<Snapshot> {
    let paths = snapshot_paths();

    if let Some(offset) = reference.strip_prefix('-') {
        let back: usize = offset.parse().context("offset must be a number")?;
        let idx = paths
            .len()
            .checked_sub(1 + back)
            .context("not that many snapshots exist")?;
        return load_snapshot(&paths[idx]);
    }

    let direct = Path::new(reference);
    if direct.exists() {
        return load_snapshot(direct);
    }

    let by_date = snapshot_dir().join(format!("{reference}.json"));
    if by_date.exists() {
        return load_snapshot(&by_date);
    }

    anyhow::bail!("no snapshot matching `{reference}`")
}

pub fn build_rollup(current: &Snapshot, previous: Option<&Snapshot>) -> Rollup {
    let accounted = current.accounted().max(1);

    let mut domains: Vec<DomainSlice> = current
        .domains
        .iter()
        .map(|d| DomainSlice {
            name: d.domain.clone(),
            bytes: d.bytes,
            pct: d.bytes as f64 * 100.0 / accounted as f64,
        })
        .collect();
    domains.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    let mut movers = Vec::new();
    if let Some(prev) = previous {
        // Recalibrating the store shifts its reported size by hundreds of
        // gigabytes overnight. That is a change of method, not of disk, and
        // reporting it as a mover would bury every real one.
        let store_comparable = current.nix_dedup.comparable_with(prev.nix_dedup);

        for domain in &current.domains {
            if domain.domain == "nix-store" && !store_comparable {
                continue;
            }
            let before = prev.domain(&domain.domain).map_or(0, |d| d.bytes);
            let delta = domain.bytes as i64 - before as i64;
            if delta != 0 {
                movers.push(Mover {
                    label: domain.domain.clone(),
                    delta,
                    delta_label: human_delta(delta),
                });
            }
        }
        movers.sort_by_key(|m| -m.delta.abs());
        movers.truncate(4);
    }

    let newcomers = current
        .domain("nix-store")
        .map(|d| {
            d.entries
                .iter()
                .filter(|e| e.newcomer)
                .take(6)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    Rollup {
        updated_at: current.taken_at,
        updated_label: current.taken_at.format("%b %d %H:%M").to_string(),
        filesystem: current.filesystem,
        pct_used: current.filesystem.pct_used(),
        delta: previous.map(|p| current.filesystem.used as i64 - p.filesystem.used as i64),
        delta_since: previous.map(|p| p.taken_at),
        domains,
        movers,
        reclaimable: current.reclaimable(),
        actions: current
            .actions()
            .into_iter()
            .take(5)
            .enumerate()
            .map(|(rank, action)| RankedAction {
                rank,
                action: action.clone(),
            })
            .collect(),
        newcomers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Dedup, DomainReport, FsStat, SNAPSHOT_VERSION};
    use chrono::Local;

    fn snapshot(used: u64, nix: u64) -> Snapshot {
        calibrated(used, nix, Dedup::Measured(0.65))
    }

    fn calibrated(used: u64, nix: u64, dedup: Dedup) -> Snapshot {
        let mut store = DomainReport::new("nix-store");
        store.bytes = nix;
        Snapshot {
            version: SNAPSHOT_VERSION,
            taken_at: Local::now(),
            filesystem: FsStat {
                total: 1_000,
                used,
                free: 1_000 - used,
            },
            domains: vec![store],
            nix_dedup: dedup,
            nix_dedup_ratio: dedup.ratio(),
            unattributed: 0,
        }
    }

    #[test]
    fn rollup_without_history_reports_no_delta() {
        let rollup = build_rollup(&snapshot(400, 400), None);
        assert!(rollup.delta.is_none());
        assert!(rollup.movers.is_empty());
    }

    #[test]
    fn rollup_delta_tracks_used_bytes_between_snapshots() {
        let previous = snapshot(300, 250);
        let rollup = build_rollup(&snapshot(400, 350), Some(&previous));
        assert_eq!(rollup.delta, Some(100));
        assert_eq!(rollup.movers.len(), 1);
        assert_eq!(rollup.movers[0].delta, 100);
    }

    #[test]
    fn rollup_delta_is_negative_when_space_is_recovered() {
        let previous = snapshot(500, 400);
        let rollup = build_rollup(&snapshot(400, 350), Some(&previous));
        assert_eq!(rollup.delta, Some(-100));
    }

    #[test]
    fn recalibrating_the_store_is_not_a_mover() {
        let previous = calibrated(300, 250, Dedup::Inferred(0.9));
        let rollup = build_rollup(&calibrated(400, 350, Dedup::Measured(0.65)), Some(&previous));
        // the filesystem really did grow, but the store's jump is method drift
        assert_eq!(rollup.delta, Some(100));
        assert!(rollup.movers.is_empty());
    }
}
