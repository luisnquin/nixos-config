use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

pub type Bytes = u64;

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Safety {
    /// Reversible or purely regenerable: caches, build artifacts, dead store paths.
    Safe,
    /// Loses history or requires a redownload to undo.
    Review,
    /// Destroys something the user cannot trivially recreate.
    Destructive,
}

/// One ranked line inside a domain: a game, a generation, a cache directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub label: String,
    pub bytes: Bytes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Unix seconds of last use, where the domain can determine it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<i64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reclaimable: Bytes,
}

impl Entry {
    pub fn new(label: impl Into<String>, bytes: Bytes) -> Self {
        Self {
            label: label.into(),
            bytes,
            detail: None,
            last_used: None,
            reclaimable: 0,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn last_used(mut self, at: Option<i64>) -> Self {
        self.last_used = at;
        self
    }

    pub fn reclaimable(mut self, bytes: Bytes) -> Self {
        self.reclaimable = bytes;
        self
    }
}

fn is_zero(v: &Bytes) -> bool {
    *v == 0
}

/// A concrete thing the user can run, with the payoff already computed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub label: String,
    pub frees: Bytes,
    pub command: String,
    pub safety: Safety,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainReport {
    pub domain: String,
    pub bytes: Bytes,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Wall-clock cost of producing this report, so slow collectors are visible.
    #[serde(default)]
    pub took_ms: u64,
}

impl DomainReport {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            bytes: 0,
            entries: Vec::new(),
            actions: Vec::new(),
            notes: Vec::new(),
            took_ms: 0,
        }
    }

    pub fn reclaimable(&self) -> Bytes {
        self.entries.iter().map(|e| e.reclaimable).sum()
    }

    /// Rank entries and keep the tail as a single aggregate line, so a snapshot
    /// stays kilobytes rather than growing with the number of store paths.
    pub fn truncate_entries(&mut self, keep: usize) {
        self.entries.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        if self.entries.len() > keep {
            let rest: Bytes = self.entries[keep..].iter().map(|e| e.bytes).sum();
            let count = self.entries.len() - keep;
            self.entries.truncate(keep);
            self.entries
                .push(Entry::new(format!("… {count} more"), rest));
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FsStat {
    pub total: Bytes,
    pub used: Bytes,
    pub free: Bytes,
}

impl FsStat {
    pub fn pct_used(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.used as f64 * 100.0 / self.total as f64
    }
}

/// NAR sizes ignore the hardlink dedup the store optimiser performs, so raw
/// sums overstate disk by ~35% on this machine. How the correction was arrived
/// at changes how much the reader — and the diff — should trust it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "ratio", rename_all = "lowercase")]
pub enum Dedup {
    /// No correction available; NAR sums reported raw.
    #[default]
    Unknown,
    /// `du(/nix/store) / sum(narSize)`, from an actual walk of the store.
    Measured(f64),
    /// Derived from what the precisely-measured domains leave unexplained in
    /// `df`. Good enough to stop the report being off by hundreds of gigabytes,
    /// but it absorbs every other accounting error, so it is never carried
    /// forward as if it had been measured.
    Inferred(f64),
}

impl Dedup {
    pub fn ratio(self) -> Option<f64> {
        match self {
            Self::Unknown => None,
            Self::Measured(r) | Self::Inferred(r) => (r > 0.0).then_some(r),
        }
    }

    /// Two calibrations of different kinds produce numbers that are not
    /// comparable, whatever their ratios happen to be.
    pub fn comparable_with(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub taken_at: DateTime<Local>,
    pub filesystem: FsStat,
    pub domains: Vec<DomainReport>,
    /// How the nix-store numbers in this snapshot were calibrated.
    #[serde(default)]
    pub nix_dedup: Dedup,
    /// Measured `du(/nix/store) / sum(narSize)`. Carried forward between deep
    /// scans because it drifts slowly but the measurement is expensive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nix_dedup_ratio: Option<f64>,
    /// Bytes the census could not attribute to any domain.
    #[serde(default)]
    pub unattributed: i64,
}

impl Snapshot {
    pub fn domain(&self, name: &str) -> Option<&DomainReport> {
        self.domains.iter().find(|d| d.domain == name)
    }

    pub fn accounted(&self) -> Bytes {
        self.domains.iter().map(|d| d.bytes).sum()
    }

    pub fn reclaimable(&self) -> Bytes {
        self.domains.iter().map(|d| d.reclaimable()).sum()
    }

    pub fn actions(&self) -> Vec<&Action> {
        let mut all: Vec<&Action> = self.domains.iter().flat_map(|d| &d.actions).collect();
        all.sort_by(|a, b| b.frees.cmp(&a.frees));
        all
    }
}

/// Precomputed view for the bar and the panel. Both read this and never scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rollup {
    pub updated_at: DateTime<Local>,
    /// Preformatted for the panel, which has no date formatting of its own and
    /// must also cope with there being no scan yet.
    #[serde(default)]
    pub updated_label: String,
    pub filesystem: FsStat,
    pub pct_used: f64,
    /// Change in *used* bytes against the previous snapshot, if there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_since: Option<DateTime<Local>>,
    pub domains: Vec<DomainSlice>,
    pub movers: Vec<Mover>,
    pub reclaimable: Bytes,
    pub actions: Vec<RankedAction>,
    pub newcomers: Vec<Entry>,
}

/// An action the panel can address by number. Copying a command to the
/// clipboard by rank keeps arbitrary command text — paths, `&&`, quotes — out
/// of the shell line that does the copying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedAction {
    pub rank: usize,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSlice {
    pub name: String,
    pub bytes: Bytes,
    pub pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mover {
    pub label: String,
    pub delta: i64,
    /// Preformatted with its sign, so the panel does not have to reimplement
    /// binary-unit rounding in simplexpr.
    #[serde(default)]
    pub delta_label: String,
}
