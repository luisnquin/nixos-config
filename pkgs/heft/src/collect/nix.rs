use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};

use crate::model::{Action, Bytes, Dedup, DomainReport, Entry, Safety};
use crate::util::{collapse_home, human, now_unix};

const DB_PATH: &str = "/nix/var/nix/db/db.sqlite";
const STORE_PREFIX: &str = "/nix/store/";

/// How many of the newest generations to try keeping, so the report can show
/// the shape of the curve rather than a single number.
const KEEP_CHECKPOINTS: [usize; 4] = [1, 5, 20, 50];

/// Profiles measured per scan. Each checkpoint costs a full graph traversal,
/// so the long tail of rarely-used profiles is not worth the seconds.
const MAX_PROFILES: usize = 3;

pub struct NixCollector {
    pub dedup: Dedup,
    pub newcomer_window_days: i64,
}

fn open_db() -> Result<Connection> {
    Connection::open_with_flags(
        DB_PATH,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .with_context(|| format!("opening {DB_PATH} read-only"))
}

/// Sum of NAR sizes without loading the reference graph. Cheap enough to run
/// before the collector, which is what lets the dedup ratio be settled up front
/// instead of scaling a finished report after the fact.
pub fn nar_total() -> Result<Bytes> {
    let total: i64 = open_db()?
        .query_row("SELECT COALESCE(SUM(narSize), 0) FROM ValidPaths", [], |row| {
            row.get(0)
        })
        .context("summing narSize")?;
    Ok(total.max(0) as Bytes)
}

/// The reference graph, remapped onto dense indices. Store ids are sparse
/// because the table is an autoincrement with deletions.
struct Graph {
    paths: Vec<String>,
    nar_size: Vec<Bytes>,
    reg_time: Vec<i64>,
    refs: Vec<Vec<u32>>,
    index: HashMap<String, u32>,
}

impl Graph {
    fn load(conn: &Connection) -> Result<Self> {
        let mut stmt = conn
            .prepare("SELECT id, path, narSize, registrationTime FROM ValidPaths")
            .context("preparing ValidPaths query")?;

        let mut paths = Vec::new();
        let mut nar_size = Vec::new();
        let mut reg_time = Vec::new();
        let mut index = HashMap::new();
        let mut dense = HashMap::new();

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;

        for row in rows {
            let (id, path, size, registered) = row?;
            let slot = paths.len() as u32;
            dense.insert(id, slot);
            index.insert(path.clone(), slot);
            paths.push(path);
            nar_size.push(size.unwrap_or(0).max(0) as Bytes);
            reg_time.push(registered);
        }

        let mut refs = vec![Vec::new(); paths.len()];
        let mut stmt = conn
            .prepare("SELECT referrer, reference FROM Refs")
            .context("preparing Refs query")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (referrer, reference) = row?;
            let (Some(&from), Some(&to)) = (dense.get(&referrer), dense.get(&reference)) else {
                continue;
            };
            if from != to {
                refs[from as usize].push(to);
            }
        }

        Ok(Self {
            paths,
            nar_size,
            reg_time,
            refs,
            index,
        })
    }

    fn total(&self) -> Bytes {
        self.nar_size.iter().sum()
    }

    /// Bytes reachable from `seeds`, following references transitively.
    fn reachable_bytes(&self, seeds: impl Iterator<Item = u32>) -> Bytes {
        let mut seen = vec![false; self.paths.len()];
        let mut stack: Vec<u32> = Vec::new();

        for seed in seeds {
            if !seen[seed as usize] {
                seen[seed as usize] = true;
                stack.push(seed);
            }
        }

        let mut total = 0;
        while let Some(node) = stack.pop() {
            total += self.nar_size[node as usize];
            for &next in &self.refs[node as usize] {
                if !seen[next as usize] {
                    seen[next as usize] = true;
                    stack.push(next);
                }
            }
        }
        total
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RootKind {
    Process {
        pid: u32,
    },
    /// A numbered generation of some profile. `profile` is the link path with
    /// the `-N-link` suffix removed, which is exactly what `nix-env --profile`
    /// expects — home-manager, `nix profile`, and the system profile all share
    /// this shape.
    Generation {
        profile: String,
        number: u32,
    },
    ActiveSystem,
    ResultLink,
    Other,
}

#[derive(Debug, Clone)]
struct Root {
    link: String,
    target: u32,
    kind: RootKind,
}

impl NixCollector {
    pub fn collect(&self) -> Result<DomainReport> {
        let mut report = DomainReport::new("nix-store");

        let conn = open_db()?;
        let graph = Graph::load(&conn)?;
        let roots = read_roots(&graph)?;

        let total = graph.total();
        let live = graph.reachable_bytes(roots.iter().map(|r| r.target));
        let dead = total.saturating_sub(live);

        let ratio = self.dedup.ratio();
        let scale = |bytes: Bytes| -> Bytes {
            match ratio {
                Some(ratio) => (bytes as f64 * ratio) as Bytes,
                None => bytes,
            }
        };

        report.bytes = scale(total);

        report.notes.push(match self.dedup {
            Dedup::Measured(ratio) => format!(
                "sizes scaled by measured dedup ratio {ratio:.3} (NAR sum is {})",
                human(total)
            ),
            Dedup::Inferred(ratio) => format!(
                "sizes scaled by inferred dedup ratio {ratio:.3} (NAR sum is {}); run \
                 `heft scan --deep` to measure it",
                human(total)
            ),
            Dedup::Unknown => format!(
                "sum of NAR sizes is {}, uncorrected for hardlink dedup; run \
                 `heft scan --deep`",
                human(total)
            ),
        });

        report.entries.push(
            Entry::new("unreachable (already garbage)", scale(dead))
                .detail("freed by a plain nix-collect-garbage")
                .reclaimable(scale(dead)),
        );

        self.push_generation_entries(&graph, &roots, &mut report, &scale);
        self.push_process_entries(&graph, &roots, &mut report, &scale);
        self.push_result_links(&graph, &roots, &mut report, &scale);
        self.push_newcomers(&graph, &mut report, &scale);

        if dead > 0 {
            report.actions.push(Action {
                label: "collect unreachable store paths".into(),
                frees: scale(dead),
                command: "nix-collect-garbage".into(),
                safety: Safety::Safe,
            });
        }

        Ok(report)
    }

    /// The counterfactual that matters: what does deleting old generations
    /// actually free, given everything else stays rooted?
    fn push_generation_entries(
        &self,
        graph: &Graph,
        roots: &[Root],
        report: &mut DomainReport,
        scale: &impl Fn(Bytes) -> Bytes,
    ) {
        let mut by_profile: HashMap<&str, Vec<(u32, u32)>> = HashMap::new();
        for root in roots {
            let RootKind::Generation {
                ref profile,
                number,
            } = root.kind
            else {
                continue;
            };
            by_profile
                .entry(profile.as_str())
                .or_default()
                .push((number, root.target));
        }

        let mut profiles: Vec<(&str, Vec<(u32, u32)>)> = by_profile
            .into_iter()
            .filter(|(_, gens)| gens.len() > 1)
            .collect();
        if profiles.is_empty() {
            return;
        }

        profiles.sort_by_key(|(_, gens)| std::cmp::Reverse(gens.len()));
        profiles.truncate(MAX_PROFILES);

        let live = graph.reachable_bytes(roots.iter().map(|r| r.target));

        for (profile, mut generations) in profiles {
            generations.sort_by(|a, b| b.0.cmp(&a.0));
            let count = generations.len();
            let name = Path::new(profile)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(profile);

            report
                .notes
                .push(format!("{count} {name} generations at {}", collapse_home(Path::new(profile))));

            let mut best: Option<(usize, Bytes)> = None;
            for keep in KEEP_CHECKPOINTS {
                if keep >= count {
                    continue;
                }
                // Retain every root except the generations being dropped; the
                // difference against the current live set is exactly what a
                // deletion would free.
                let dropped: HashSet<u32> =
                    generations[keep..].iter().map(|(_, t)| *t).collect();
                let retained = roots
                    .iter()
                    .map(|r| r.target)
                    .filter(|t| !dropped.contains(t));
                let freed = live.saturating_sub(graph.reachable_bytes(retained));

                report.entries.push(
                    Entry::new(
                        format!("drop {name} generations past newest {keep}"),
                        scale(freed),
                    )
                    .detail(format!("{} generations removed", count - keep)),
                );

                if best.map_or(true, |(_, b)| freed > b) {
                    best = Some((keep, freed));
                }
            }

            if let Some((keep, freed)) = best {
                if freed > 0 {
                    report.actions.push(Action {
                        label: format!("prune {name} generations, keeping newest {keep}"),
                        frees: scale(freed),
                        // nix-env, not `nix profile wipe-history`: these are
                        // nix-env-style profiles, and `--older-than` takes a
                        // duration where what is wanted is a count.
                        command: format!(
                            "nix-env --profile {profile} --delete-generations +{keep}"
                        ),
                        safety: Safety::Review,
                    });
                }
            }
        }
    }

    /// Roots under /proc are live processes. They cannot be deleted, but they
    /// explain store paths that look unaccounted for.
    fn push_process_entries(
        &self,
        graph: &Graph,
        roots: &[Root],
        report: &mut DomainReport,
        scale: &impl Fn(Bytes) -> Bytes,
    ) {
        let mut by_comm: HashMap<String, Vec<u32>> = HashMap::new();
        for root in roots {
            let RootKind::Process { pid } = root.kind else {
                continue;
            };
            by_comm.entry(process_name(pid)).or_default().push(root.target);
        }

        if by_comm.is_empty() {
            return;
        }

        // Each candidate costs a full traversal, so only the process names
        // holding the most roots are worth measuring.
        let mut candidates: Vec<(String, Vec<u32>)> = by_comm.into_iter().collect();
        candidates.sort_by_key(|(_, targets)| std::cmp::Reverse(targets.len()));
        candidates.truncate(12);

        let non_process: Vec<u32> = roots
            .iter()
            .filter(|r| !matches!(r.kind, RootKind::Process { .. }))
            .map(|r| r.target)
            .collect();
        let baseline = graph.reachable_bytes(non_process.iter().copied());

        let mut pinned: Vec<(String, Bytes)> = candidates
            .into_iter()
            .map(|(comm, targets)| {
                let with = graph
                    .reachable_bytes(non_process.iter().copied().chain(targets.iter().copied()));
                (comm, with.saturating_sub(baseline))
            })
            .filter(|(_, bytes)| *bytes > 0)
            .collect();

        pinned.sort_by(|a, b| b.1.cmp(&a.1));
        pinned.truncate(5);

        for (comm, bytes) in pinned {
            report.entries.push(
                Entry::new(format!("pinned by running {comm}"), scale(bytes))
                    .detail("freed only when the process exits"),
            );
        }
    }

    fn push_result_links(
        &self,
        graph: &Graph,
        roots: &[Root],
        report: &mut DomainReport,
        scale: &impl Fn(Bytes) -> Bytes,
    ) {
        let links: Vec<&Root> = roots
            .iter()
            .filter(|r| r.kind == RootKind::ResultLink)
            .collect();
        if links.is_empty() {
            return;
        }

        let live = graph.reachable_bytes(roots.iter().map(|r| r.target));
        let dropped: Vec<u32> = links.iter().map(|r| r.target).collect();
        let retained = roots
            .iter()
            .map(|r| r.target)
            .filter(|t| !dropped.contains(t));
        let freed = live.saturating_sub(graph.reachable_bytes(retained));

        if freed == 0 {
            return;
        }

        report.entries.push(
            Entry::new(
                format!("{} stray `result` symlinks", links.len()),
                scale(freed),
            )
            .detail(
                links
                    .iter()
                    .take(3)
                    .map(|r| r.link.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .reclaimable(scale(freed)),
        );

        report.actions.push(Action {
            label: format!("remove {} `result` symlinks", links.len()),
            frees: scale(freed),
            // Every link, not a sample: a command that covers less than its
            // label claims frees less than the number printed beside it.
            command: std::iter::once("rm".to_string())
                .chain(links.iter().map(|r| r.link.clone()))
                .collect::<Vec<_>>()
                .join(" "),
            safety: Safety::Review,
        });
    }

    /// `registrationTime` means "what is new" is a query, not a snapshot diff.
    fn push_newcomers(
        &self,
        graph: &Graph,
        report: &mut DomainReport,
        scale: &impl Fn(Bytes) -> Bytes,
    ) {
        let cutoff = now_unix() - self.newcomer_window_days * 86_400;
        let mut fresh: Vec<(Bytes, &str)> = graph
            .reg_time
            .iter()
            .enumerate()
            .filter(|(_, &t)| t >= cutoff)
            .map(|(i, _)| (graph.nar_size[i], graph.paths[i].as_str()))
            .collect();

        if fresh.is_empty() {
            return;
        }

        fresh.sort_by(|a, b| b.0.cmp(&a.0));
        let total: Bytes = fresh.iter().map(|(b, _)| b).sum();
        report.notes.push(format!(
            "{} paths registered in the last {} day(s), {} total",
            fresh.len(),
            self.newcomer_window_days,
            human(scale(total))
        ));

        for (bytes, path) in fresh.into_iter().take(8) {
            report
                .entries
                .push(Entry::new(format!("new: {}", store_name(path)), scale(bytes)));
        }
    }
}

fn read_roots(graph: &Graph) -> Result<Vec<Root>> {
    let output = Command::new("nix-store")
        .args(["--gc", "--print-roots"])
        .output()
        .context("running `nix-store --gc --print-roots`")?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut roots = Vec::new();

    for line in text.lines() {
        let Some((link, target)) = line.split_once(" -> ") else {
            continue;
        };
        let link = link.trim().trim_matches('"');
        let target = target.trim();
        if link.starts_with('{') {
            // censored roots carry no usable path
            continue;
        }
        let Some(&id) = graph.index.get(top_level_store_path(target)) else {
            continue;
        };
        roots.push(Root {
            link: link.to_string(),
            target: id,
            kind: classify(link),
        });
    }

    Ok(roots)
}

/// Roots may point inside a store path; attribution needs the top-level entry.
fn top_level_store_path(path: &str) -> &str {
    let Some(rest) = path.strip_prefix(STORE_PREFIX) else {
        return path;
    };
    match rest.find('/') {
        Some(idx) => &path[..STORE_PREFIX.len() + idx],
        None => path,
    }
}

fn store_name(path: &str) -> &str {
    let name = path.strip_prefix(STORE_PREFIX).unwrap_or(path);
    // strip the 32-character base32 hash and its separating dash
    name.get(33..).unwrap_or(name)
}

fn classify(link: &str) -> RootKind {
    if let Some(rest) = link.strip_prefix("/proc/") {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(pid) = digits.parse() {
            return RootKind::Process { pid };
        }
        return RootKind::Other;
    }

    if link == "/run/current-system" || link == "/run/booted-system" {
        return RootKind::ActiveSystem;
    }

    let path = Path::new(link);
    let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if let Some((stem, number)) = generation_link(file) {
        let profile = match path.parent() {
            Some(dir) => dir.join(stem).to_string_lossy().into_owned(),
            None => stem.to_string(),
        };
        return RootKind::Generation { profile, number };
    }
    if file == "result" || file.starts_with("result-") {
        return RootKind::ResultLink;
    }

    RootKind::Other
}

/// `home-manager-217-link` -> `("home-manager", 217)`. The same shape covers
/// `profile-N-link`, `system-N-link` and `channels-N-link`, and the stem is
/// exactly the profile name `nix-env --profile` expects.
fn generation_link(file: &str) -> Option<(&str, u32)> {
    let rest = file.strip_suffix("-link")?;
    let (stem, digits) = rest.rsplit_once('-')?;
    if stem.is_empty() {
        return None;
    }
    Some((stem, digits.parse().ok()?))
}

fn process_name(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| format!("pid {pid}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_paths_normalise_to_their_top_level_entry() {
        assert_eq!(
            top_level_store_path("/nix/store/abc-foo/bin/bar"),
            "/nix/store/abc-foo"
        );
        assert_eq!(
            top_level_store_path("/nix/store/abc-foo"),
            "/nix/store/abc-foo"
        );
    }

    #[test]
    fn store_name_drops_the_hash() {
        assert_eq!(
            store_name("/nix/store/3q6zfblbx7g4whmwp0xvlwm61qh53632-flake-registry.json"),
            "flake-registry.json"
        );
    }

    #[test]
    fn roots_classify_by_their_link_path() {
        assert_eq!(classify("/proc/3316/maps"), RootKind::Process { pid: 3316 });
        assert_eq!(classify("/run/current-system"), RootKind::ActiveSystem);
        assert_eq!(
            classify("/home/u/.local/state/nix/profiles/home-manager-100-link"),
            RootKind::Generation {
                profile: "/home/u/.local/state/nix/profiles/home-manager".into(),
                number: 100
            }
        );
        assert_eq!(
            classify("/home/u/.local/state/nix/profiles/profile-42-link"),
            RootKind::Generation {
                profile: "/home/u/.local/state/nix/profiles/profile".into(),
                number: 42
            }
        );
        assert_eq!(
            classify("/nix/var/nix/profiles/system-5-link"),
            RootKind::Generation {
                profile: "/nix/var/nix/profiles/system".into(),
                number: 5
            }
        );
        assert_eq!(classify("/home/u/Projects/thing/result"), RootKind::ResultLink);
        assert_eq!(
            classify("/home/u/Projects/thing/result-iso-jp7-1"),
            RootKind::ResultLink
        );
        assert_eq!(classify("/home/u/.cache/nix/flake-registry.json"), RootKind::Other);
    }

    #[test]
    fn generation_links_need_both_a_stem_and_a_number() {
        assert_eq!(generation_link("home-manager-3-link"), Some(("home-manager", 3)));
        assert_eq!(generation_link("-3-link"), None);
        assert_eq!(generation_link("home-manager-link"), None);
        assert_eq!(generation_link("result"), None);
    }

    #[test]
    fn reachability_follows_references_transitively() {
        let graph = Graph {
            paths: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            nar_size: vec![10, 20, 40, 80],
            reg_time: vec![0; 4],
            refs: vec![vec![1], vec![2], vec![], vec![]],
            index: HashMap::new(),
        };
        assert_eq!(graph.reachable_bytes([0u32].into_iter()), 70);
        assert_eq!(graph.reachable_bytes([3u32].into_iter()), 80);
        assert_eq!(graph.total(), 150);
    }

    #[test]
    fn reachability_terminates_on_cycles() {
        let graph = Graph {
            paths: vec!["a".into(), "b".into()],
            nar_size: vec![10, 20],
            reg_time: vec![0; 2],
            refs: vec![vec![1], vec![0]],
            index: HashMap::new(),
        };
        assert_eq!(graph.reachable_bytes([0u32].into_iter()), 30);
    }
}
