use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::thread;

use anyhow::Result;

use crate::model::{DomainReport, Entry};
use crate::util::{collapse_home, expand_home, human, Walker};

/// Everything the metadata-driven collectors do not claim. This is the only
/// collector that must actually walk the filesystem, and by construction it is
/// the smallest slice — which is what keeps a full scan affordable.
pub struct ResidualCollector {
    pub roots: Vec<String>,
    pub claimed: Vec<PathBuf>,
    pub cold_days: i64,
}

impl Default for ResidualCollector {
    fn default() -> Self {
        Self {
            roots: vec!["~".into(), "/var".into()],
            claimed: Vec::new(),
            cold_days: 90,
        }
    }
}

impl ResidualCollector {
    pub fn collect(&self) -> Result<DomainReport> {
        let mut report = DomainReport::new("residual");

        let skip: HashSet<PathBuf> = self.claimed.iter().cloned().collect();
        let mut targets = Vec::new();

        for root in &self.roots {
            let root = PathBuf::from(expand_home(root));
            if !root.is_dir() {
                continue;
            }
            // Descend one level so the report names a directory the user
            // recognises rather than a single opaque total for $HOME.
            let Ok(entries) = fs::read_dir(&root) else {
                targets.push(root);
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if skip.contains(&path) {
                    continue;
                }
                targets.push(path);
            }
        }

        // One thread per top-level directory: the walk is IO-bound and the
        // trees are disjoint, so there is nothing to synchronise.
        let cold_days = self.cold_days;
        let results: Vec<(PathBuf, crate::util::WalkResult)> = thread::scope(|scope| {
            let handles: Vec<_> = targets
                .iter()
                .map(|path| {
                    let skip = &skip;
                    scope.spawn(move || {
                        let mut walker = Walker::new(cold_days).skipping(skip.iter().cloned());
                        (path.clone(), walker.walk(path))
                    })
                })
                .collect();

            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });

        for (path, result) in results {
            if result.bytes == 0 {
                continue;
            }
            report.bytes += result.bytes;
            report.entries.push(
                Entry::new(collapse_home(&path), result.bytes)
                    .detail(format!("{} files · {} cold", result.files, human(result.cold)))
                    .last_used(Some(result.newest).filter(|t| *t > 0)),
            );
        }

        report.truncate_entries(12);
        Ok(report)
    }
}
