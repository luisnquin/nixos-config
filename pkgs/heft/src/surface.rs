use serde::Serialize;

use crate::model::{Bytes, Rollup};
use crate::util::{human, human_delta};

#[derive(Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
    pub class: Vec<String>,
    pub percentage: u64,
}

pub struct Thresholds {
    pub warn_free: Bytes,
    pub critical_free: Bytes,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            warn_free: 100 * 1024 * 1024 * 1024,
            critical_free: 50 * 1024 * 1024 * 1024,
        }
    }
}

pub fn waybar(rollup: &Rollup, thresholds: &Thresholds) -> WaybarOutput {
    let free = rollup.filesystem.free;

    // Deliberately not `warning`/`critical`: the bar stylesheet blinks those
    // classes on sight, and a disk sits near its limit for weeks at a time.
    let class = if free <= thresholds.critical_free {
        "cramped"
    } else if free <= thresholds.warn_free {
        "tight"
    } else {
        "ample"
    };

    let mut lines = vec![format!(
        "Disk: {} free of {} · {:.0}% used",
        human(free),
        human(rollup.filesystem.total),
        rollup.pct_used
    )];

    if let (Some(delta), Some(since)) = (rollup.delta, rollup.delta_since) {
        lines.push(format!(
            "{} since {}",
            human_delta(delta),
            since.format("%b %d")
        ));
    }

    for mover in rollup.movers.iter().take(3) {
        lines.push(format!("{}  {}", mover.delta_label, mover.label));
    }

    if rollup.reclaimable > 0 {
        lines.push(format!("{} reclaimable", human(rollup.reclaimable)));
    }

    if let Some(top) = rollup.actions.first() {
        lines.push(format!(
            "top: {} (+{})",
            top.action.label,
            human(top.action.frees)
        ));
    }

    lines.push(format!("updated {}", rollup.updated_label));

    WaybarOutput {
        text: " ".into(),
        tooltip: escape_pango(&lines.join("\n")),
        class: vec![
            class.into(),
            format!(
                "level-{}",
                (rollup.pct_used * 16.0 / 100.0).round().clamp(0.0, 16.0) as u8
            ),
        ],
        percentage: rollup.pct_used.round().clamp(0.0, 100.0) as u64,
    }
}

/// Waybar renders tooltips as Pango markup, so a stray `&` or `<` in a game
/// title breaks the whole bar module.
fn escape_pango(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FsStat;
    use chrono::Local;

    fn rollup(free: Bytes) -> Rollup {
        Rollup {
            updated_at: Local::now(),
            updated_label: "Aug 13 19:40".into(),
            filesystem: FsStat {
                total: 1000 * 1024 * 1024 * 1024,
                used: 1000 * 1024 * 1024 * 1024 - free,
                free,
            },
            pct_used: 50.0,
            delta: None,
            delta_since: None,
            domains: Vec::new(),
            movers: Vec::new(),
            reclaimable: 0,
            actions: Vec::new(),
            newcomers: Vec::new(),
        }
    }

    #[test]
    fn class_escalates_as_free_space_falls() {
        let t = Thresholds::default();
        assert_eq!(
            waybar(&rollup(400 * 1024 * 1024 * 1024), &t).class[0],
            "ample"
        );
        assert_eq!(
            waybar(&rollup(80 * 1024 * 1024 * 1024), &t).class[0],
            "tight"
        );
        assert_eq!(
            waybar(&rollup(20 * 1024 * 1024 * 1024), &t).class[0],
            "cramped"
        );
    }

    #[test]
    fn level_is_used_share_in_sixteenths() {
        let t = Thresholds::default();
        let mut r = rollup(400 * 1024 * 1024 * 1024);
        for (pct, level) in [
            (0.0, "level-0"),
            (9.0, "level-1"),
            (80.0, "level-13"),
            (100.0, "level-16"),
        ] {
            r.pct_used = pct;
            assert_eq!(waybar(&r, &t).class[1], level);
        }
    }

    #[test]
    fn pango_metacharacters_are_escaped() {
        assert_eq!(escape_pango("Tom & Jerry <3"), "Tom &amp; Jerry &lt;3");
    }
}
