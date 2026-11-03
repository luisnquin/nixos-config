use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeDelta};

use crate::model::{Bytes, FsStat, Snapshot};

const GIB: Bytes = 1024 * 1024 * 1024;

const MAX_MEMORY_STALL: f64 = 10.0;
const MAX_DISK_BUSY: f64 = 0.5;

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Rest,
    Census,
    Reclaim,
}

pub struct Policy {
    pub warn_free: Bytes,
    pub moved: Bytes,
    pub max_age: TimeDelta,
}

impl Policy {
    pub fn new(warn_free_gib: u64) -> Self {
        Self {
            warn_free: warn_free_gib * GIB,
            moved: 5 * GIB,
            max_age: TimeDelta::hours(24),
        }
    }
}

pub fn decide(
    live: FsStat,
    last: Option<&Snapshot>,
    now: DateTime<Local>,
    policy: &Policy,
) -> Decision {
    if live.free <= policy.warn_free {
        return Decision::Reclaim;
    }
    let Some(last) = last else {
        return Decision::Census;
    };
    if live.used.abs_diff(last.filesystem.used) >= policy.moved
        || now - last.taken_at >= policy.max_age
    {
        Decision::Census
    } else {
        Decision::Rest
    }
}

pub fn busy(filesystem: &str) -> Result<Option<String>> {
    let memory = fs::read_to_string("/proc/pressure/memory").context("reading memory PSI")?;
    if let Some(stall) = psi_some_avg60(&memory) {
        if stall > MAX_MEMORY_STALL {
            return Ok(Some(format!(
                "memory stalled {stall:.0}% of the last minute"
            )));
        }
    }

    if let Some(util) = disk_utilisation(filesystem, Duration::from_secs(2)) {
        if util > MAX_DISK_BUSY {
            return Ok(Some(format!(
                "disk behind {filesystem} {:.0}% busy",
                util * 100.0
            )));
        }
    }
    Ok(None)
}

fn psi_some_avg60(text: &str) -> Option<f64> {
    text.lines()
        .find(|l| l.starts_with("some "))?
        .split_whitespace()
        .find_map(|f| f.strip_prefix("avg60="))?
        .parse()
        .ok()
}

fn disk_utilisation(path: &str, window: Duration) -> Option<f64> {
    let dev = rustix::fs::stat(path).ok()?.st_dev;
    let stat = format!(
        "/sys/dev/block/{}:{}/stat",
        rustix::fs::major(dev),
        rustix::fs::minor(dev)
    );
    let started = Instant::now();
    let before = io_ticks(&fs::read_to_string(&stat).ok()?)?;
    thread::sleep(window);
    let after = io_ticks(&fs::read_to_string(&stat).ok()?)?;
    Some(after.saturating_sub(before) as f64 / started.elapsed().as_millis().max(1) as f64)
}

fn io_ticks(stat: &str) -> Option<u64> {
    stat.split_whitespace().nth(9)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Dedup, SNAPSHOT_VERSION};

    fn fs(free: Bytes) -> FsStat {
        FsStat {
            total: 1000 * GIB,
            used: 1000 * GIB - free,
            free,
        }
    }

    fn snapshot(free: Bytes, age: TimeDelta) -> Snapshot {
        Snapshot {
            version: SNAPSHOT_VERSION,
            taken_at: Local::now() - age,
            filesystem: fs(free),
            domains: Vec::new(),
            nix_dedup: Dedup::Unknown,
            nix_dedup_ratio: None,
            unattributed: 0,
        }
    }

    #[test]
    fn decision_follows_free_space_drift_and_age() {
        let policy = Policy::new(100);
        let now = Local::now();
        let fresh = snapshot(300 * GIB, TimeDelta::hours(1));

        assert_eq!(decide(fs(300 * GIB), None, now, &policy), Decision::Census);
        assert_eq!(
            decide(fs(302 * GIB), Some(&fresh), now, &policy),
            Decision::Rest
        );
        assert_eq!(
            decide(fs(290 * GIB), Some(&fresh), now, &policy),
            Decision::Census
        );
        assert_eq!(
            decide(fs(310 * GIB), Some(&fresh), now, &policy),
            Decision::Census
        );
        let stale = snapshot(300 * GIB, TimeDelta::hours(25));
        assert_eq!(
            decide(fs(300 * GIB), Some(&stale), now, &policy),
            Decision::Census
        );
        assert_eq!(
            decide(fs(90 * GIB), Some(&fresh), now, &policy),
            Decision::Reclaim
        );
    }

    #[test]
    fn kernel_counters_parse() {
        let psi = "some avg10=1.50 avg60=12.25 avg300=3.00 total=9\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(psi_some_avg60(psi), Some(12.25));
        let stat = " 3815811  2100064 204608731   566173   810225  2878192 40865946  6675433        0   442073  7246420";
        assert_eq!(io_ticks(stat), Some(442073));
    }
}
