use crate::model::{Safety, Snapshot};
use crate::util::{human, human_delta};

const BAR_WIDTH: usize = 28;

struct Style {
    enabled: bool,
}

impl Style {
    fn detect() -> Self {
        Self {
            enabled: std::env::var_os("NO_COLOR").is_none(),
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }

    fn bold(&self, text: &str) -> String {
        self.paint("1", text)
    }

    fn severity(&self, pct: f64, text: &str) -> String {
        let code = if pct >= 90.0 {
            "31"
        } else if pct >= 75.0 {
            "33"
        } else {
            "32"
        };
        self.paint(code, text)
    }
}

pub fn render(snapshot: &Snapshot, previous: Option<&Snapshot>) -> String {
    let style = Style::detect();
    let mut out = String::new();

    let fs = snapshot.filesystem;
    let pct = fs.pct_used();

    out.push_str(&format!(
        "\n{} {}\n\n",
        style.bold("heft"),
        style.dim(&snapshot.taken_at.format("%Y-%m-%d %H:%M").to_string())
    ));

    out.push_str(&format!(
        "  {} total · {} used · {} free · {}\n",
        human(fs.total),
        human(fs.used),
        style.severity(pct, &human(fs.free)),
        style.severity(pct, &format!("{pct:.0}%"))
    ));

    if let Some(prev) = previous {
        let delta = fs.used as i64 - prev.filesystem.used as i64;
        out.push_str(&format!(
            "  {} since {}\n",
            human_delta(delta),
            prev.taken_at.format("%Y-%m-%d")
        ));
    }

    out.push('\n');

    let accounted = snapshot.accounted().max(1);
    let mut domains: Vec<_> = snapshot.domains.iter().collect();
    domains.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    for domain in &domains {
        let share = domain.bytes as f64 / accounted as f64;
        out.push_str(&format!(
            "  {:<14} {} {:>6}  {:>3.0}%\n",
            domain.domain,
            bar(share, &style),
            human(domain.bytes),
            share * 100.0
        ));
    }

    if snapshot.unattributed.abs() > (fs.total / 100) as i64 {
        out.push_str(&format!(
            "  {}\n",
            style.dim(&format!(
                "{:<14} {} unaccounted (other mounts, /usr, /run)",
                "",
                human_delta(snapshot.unattributed)
            ))
        ));
    }

    let reclaimable = snapshot.reclaimable();
    if reclaimable > 0 {
        out.push_str(&format!(
            "\n  {} {}\n",
            style.bold(&human(reclaimable)),
            style.dim("reclaimable")
        ));
    }

    for domain in &domains {
        if domain.entries.is_empty() && domain.notes.is_empty() {
            continue;
        }

        out.push_str(&format!(
            "\n{}  {}\n",
            style.bold(&domain.domain.to_uppercase()),
            style.dim(&format!("{} · {}ms", human(domain.bytes), domain.took_ms))
        ));

        for note in &domain.notes {
            out.push_str(&format!("  {}\n", style.dim(note)));
        }

        for entry in domain.entries.iter().take(10) {
            let detail = entry
                .detail
                .as_deref()
                .map(|d| format!("  {}", style.dim(d)))
                .unwrap_or_default();
            out.push_str(&format!(
                "  {:>7}  {}{}{}\n",
                human(entry.bytes),
                if entry.newcomer { "new: " } else { "" },
                entry.label,
                detail
            ));
        }
    }

    let actions = snapshot.actions();
    if !actions.is_empty() {
        out.push_str(&format!("\n{}\n", style.bold("ACTIONS")));
        for (idx, action) in actions.iter().enumerate() {
            let tag = match action.safety {
                Safety::Safe => style.paint("32", "safe"),
                Safety::Review => style.paint("33", "review"),
                Safety::Destructive => style.paint("31", "destructive"),
            };
            out.push_str(&format!(
                "  {}. {}  {} {}  [{}]\n     {}\n",
                idx + 1,
                action.label,
                style.dim("frees"),
                human(action.frees),
                tag,
                style.dim(&action.command)
            ));
        }
    }

    out.push('\n');
    out
}

pub fn render_diff(before: &Snapshot, after: &Snapshot) -> String {
    let style = Style::detect();
    let mut out = String::new();

    out.push_str(&format!(
        "\n{} {} → {}\n\n",
        style.bold("heft diff"),
        before.taken_at.format("%Y-%m-%d %H:%M"),
        after.taken_at.format("%Y-%m-%d %H:%M")
    ));

    let delta = after.filesystem.used as i64 - before.filesystem.used as i64;
    out.push_str(&format!(
        "  used {} → {}  ({})\n\n",
        human(before.filesystem.used),
        human(after.filesystem.used),
        human_delta(delta)
    ));

    let mut rows: Vec<(String, i64)> = after
        .domains
        .iter()
        .map(|d| {
            let was = before.domain(&d.domain).map_or(0, |p| p.bytes);
            (d.domain.clone(), d.bytes as i64 - was as i64)
        })
        .collect();
    rows.sort_by_key(|(_, delta)| -delta.abs());

    for (domain, delta) in rows {
        if delta == 0 {
            continue;
        }
        let text = human_delta(delta);
        let painted = if delta > 0 {
            style.paint("31", &text)
        } else {
            style.paint("32", &text)
        };
        out.push_str(&format!("  {painted:>18}  {domain}\n"));
    }

    out.push('\n');
    out
}

fn bar(share: f64, style: &Style) -> String {
    let filled = (share * BAR_WIDTH as f64).round().clamp(0.0, BAR_WIDTH as f64) as usize;
    let full = "█".repeat(filled);
    let empty = "░".repeat(BAR_WIDTH - filled);
    format!("{}{}", full, style.dim(&empty))
}
