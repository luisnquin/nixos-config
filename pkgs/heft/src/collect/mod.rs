pub mod buildcache;
pub mod nix;
pub mod residual;
pub mod steam;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use chrono::Local;

use crate::model::{Dedup, DomainReport, Snapshot, SNAPSHOT_VERSION};
use crate::util::{expand_home, statvfs, Walker};

pub struct Config {
    pub steam_root: String,
    pub build_cache_paths: Vec<String>,
    pub residual_roots: Vec<String>,
    pub cold_days: i64,
    pub stale_days: i64,
    pub newcomer_window_days: i64,
    pub filesystem: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            steam_root: "~/.local/share/Steam".into(),
            build_cache_paths: buildcache::DEFAULT_PATHS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            residual_roots: vec!["~".into(), "/var".into()],
            cold_days: 90,
            stale_days: 180,
            newcomer_window_days: 1,
            // via the bind mount this reaches the disk that actually holds the
            // store and home, even when / itself is a tmpfs
            filesystem: "/nix".into(),
        }
    }
}

/// Run every collector and assemble a census.
///
/// `dedup_ratio` is carried forward from the previous snapshot unless `deep` is
/// set, because measuring it means walking the whole store.
pub fn census(config: &Config, carried_ratio: Option<f64>, deep: bool) -> Result<Snapshot> {
    let filesystem = statvfs(&config.filesystem)?;

    let steam = steam::SteamCollector {
        root: PathBuf::from(expand_home(&config.steam_root)),
        stale_days: config.stale_days,
    };
    let caches = buildcache::BuildCacheCollector {
        paths: config.build_cache_paths.clone(),
        cold_days: config.cold_days,
    };

    let mut claimed = caches.expanded();
    claimed.push(steam.root.clone());

    let residual = residual::ResidualCollector {
        roots: config.residual_roots.clone(),
        claimed,
        cold_days: config.cold_days,
    };

    // Everything but the store first: these are measured directly, so their
    // sum is what makes the store's share of `df` inferable below.
    let mut domains = vec![
        timed(|| steam.collect())?,
        timed(|| caches.collect())?,
        timed(|| residual.collect())?,
    ];

    let nar_total = nix::nar_total()?;
    let measured = if deep {
        measure_dedup_ratio(nar_total)
    } else {
        carried_ratio
    };

    let dedup = match measured {
        Some(ratio) => Dedup::Measured(ratio),
        None => {
            let others: u64 = domains.iter().map(|d| d.bytes).sum();
            match infer_dedup_ratio(nar_total, filesystem.used, others) {
                Some(ratio) => Dedup::Inferred(ratio),
                None => Dedup::Unknown,
            }
        }
    };

    let mut nix_report = timed(|| {
        nix::NixCollector {
            dedup,
            newcomer_window_days: config.newcomer_window_days,
        }
        .collect()
    })?;
    nix_report.truncate_entries(14);
    domains.insert(0, nix_report);

    let accounted: u64 = domains.iter().map(|d| d.bytes).sum();
    let unattributed = filesystem.used as i64 - accounted as i64;

    Ok(Snapshot {
        version: SNAPSHOT_VERSION,
        taken_at: Local::now(),
        filesystem,
        domains,
        nix_dedup: dedup,
        // Only a measured ratio is persisted. Carrying an inferred one forward
        // would launder a guess into a measurement on the next scan.
        nix_dedup_ratio: measured,
        unattributed,
    })
}

fn timed(run: impl FnOnce() -> Result<DomainReport>) -> Result<DomainReport> {
    let started = Instant::now();
    let mut report = run()?;
    report.took_ms = started.elapsed().as_millis() as u64;
    Ok(report)
}

/// NAR sizes ignore the hardlink dedup the store optimiser performs. The only
/// honest correction is to measure both and divide — and the ratio drifts, so
/// it is a per-scan measurement rather than a constant.
fn measure_dedup_ratio(nar_total: u64) -> Option<f64> {
    if nar_total == 0 {
        return None;
    }
    let mut walker = Walker::new(0);
    let on_disk = walker.walk(std::path::Path::new("/nix/store")).bytes;
    if on_disk == 0 {
        return None;
    }
    Some(on_disk as f64 / nar_total as f64)
}

/// Standing in for a measurement: whatever `df` reports as used and the other
/// domains do not explain must be the store. Rejected outright when it lands
/// outside `(0, 1]` — dedup can only shrink the NAR sum, so a ratio above one
/// means the residual is polluted rather than that the store is larger.
fn infer_dedup_ratio(nar_total: u64, used: u64, others: u64) -> Option<f64> {
    if nar_total == 0 {
        return None;
    }
    let ratio = used.checked_sub(others)? as f64 / nar_total as f64;
    (ratio > 0.0 && ratio <= 1.0).then_some(ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_rejects_implausible_ratios() {
        assert_eq!(infer_dedup_ratio(1000, 900, 250), Some(0.65));
        // more residual than df reports used
        assert_eq!(infer_dedup_ratio(1000, 200, 400), None);
        // dedup cannot make the store larger than its NAR sum
        assert_eq!(infer_dedup_ratio(1000, 2000, 0), None);
        assert_eq!(infer_dedup_ratio(0, 900, 250), None);
    }
}
