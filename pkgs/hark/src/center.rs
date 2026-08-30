use crate::config::Config;
use crate::format::{clamp_lines, clock, now_ms, plural, preview, relative, truncate};
use crate::mako::{self, Notification, DND_MODE};
use crate::slug;
use serde_json::{json, Value};

pub const DEFAULT_ICON: &str = "\u{f009a}";

pub struct Group {
    pub key: String,
    label: String,
    icon: String,
    order: i64,
    always_collapse: bool,
    pub entries: Vec<Notification>,
}

pub struct Centre {
    pub groups: Vec<Group>,
    pub total: usize,
    pub active: usize,
    pub unread: usize,
    pub critical: usize,
    pub unread_critical: usize,
    pub dnd: bool,
    pub daemon: bool,
    seen_at: u64,
    now: u64,
}

pub fn build(config: &Config, seen_at: u64) -> Centre {
    let daemon = mako::running();
    let dnd = mako::modes().iter().any(|mode| mode == DND_MODE);
    let notifications = mako::collect().unwrap_or_default();
    let now = now_ms();

    let mut groups: Vec<Group> = Vec::new();
    let mut total = 0;
    let mut active = 0;
    let mut unread = 0;
    let mut critical = 0;
    let mut unread_critical = 0;

    for notification in notifications {
        if config.ignores(&notification) {
            continue;
        }
        total += 1;
        active += usize::from(notification.active);
        let fresh = notification.created_at > seen_at;
        unread += usize::from(fresh);
        critical += usize::from(notification.is_critical());
        unread_critical += usize::from(fresh && notification.is_critical());

        let (key, label, icon, order, always_collapse) = match config.rule_for(&notification) {
            Some(rule) => (
                rule.key.clone(),
                rule.label.clone(),
                rule.icon.clone(),
                rule.priority,
                rule.always_collapse,
            ),
            None if config.group_rest_by_app => {
                let source = notification.source().to_owned();
                (slug(&source), source, String::new(), 1000, false)
            }
            None => ("everything".to_owned(), "Everything else".to_owned(), String::new(), 1000, false),
        };

        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => group.entries.push(notification),
            None => groups.push(Group {
                key,
                label,
                icon,
                order,
                always_collapse,
                entries: vec![notification],
            }),
        }
    }

    // Newest first, both between groups and inside them: a centre is read from
    // the top, and the declared priority only breaks ties.
    for group in &mut groups {
        group.entries.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));
    }
    groups.sort_by(|a, b| {
        let recency = |group: &Group| group.entries.first().map_or(0, |entry| entry.created_at);
        recency(b)
            .cmp(&recency(a))
            .then(a.order.cmp(&b.order))
            .then(a.label.cmp(&b.label))
    });

    Centre {
        groups,
        total,
        active,
        unread,
        critical,
        unread_critical,
        dnd,
        daemon,
        seen_at,
        now,
    }
}

// Rough per-row heights of the collapsed layout. The scroll viewport has to be
// given a pixel height up front, so the model measures what it is about to
// render instead of leaving the panel padded out to a guess.
const GROUP_HEAD: usize = 38;
const PREVIEW_ROW: usize = 22;
const ENTRY_ROW: usize = 84;
const GROUP_GAP: usize = 8;

impl Centre {
    pub fn model(&self, config: &Config) -> Value {
        json!({
            "daemon": self.daemon,
            "dnd": self.dnd,
            "empty": self.total == 0,
            "total": self.total,
            "active": self.active,
            "unread": self.unread,
            "critical": self.critical,
            "unread_critical": self.unread_critical,
            "headline": self.headline(),
            "panel_height": self.panel_height(config),
            // eww has no list literal, so anything the panel renders
            // conditionally arrives as a zero- or one-element array and goes
            // through `for`. `none` is the empty side of every such ternary.
            "none": [],
            "void_rows": if self.total == 0 { vec![self.headline()] } else { Vec::new() },
            "groups": self.groups
                .iter()
                .map(|group| self.group_model(group, config))
                .collect::<Vec<_>>(),
        })
    }

    pub fn waybar(&self, config: &Config) -> Value {
        let (icon, class) = if !self.daemon {
            ("\u{f009b}", "down")
        } else if self.dnd {
            ("\u{f009b}", "dnd")
        } else if self.unread_critical > 0 {
            ("\u{f01ee}", "alert")
        } else if self.unread > 0 {
            ("\u{f01ee}", "unread")
        } else {
            ("\u{f01f0}", "empty")
        };

        // The badge counts what arrived since the centre was last opened; the
        // tooltip carries the whole backlog, which is the number that never
        // drops on its own.
        let text = if self.unread > 0 {
            format!("{icon} <span size=\"8pt\">{}</span>", self.unread)
        } else {
            icon.to_owned()
        };

        json!({
            "text": text,
            "alt": class,
            "class": class,
            "tooltip": self.tooltip(config),
        })
    }

    fn panel_height(&self, config: &Config) -> usize {
        let mut height = 0;
        for group in &self.groups {
            height += GROUP_HEAD + GROUP_GAP;
            if self.open_by_default(group, config) {
                height += group.entries.len() * ENTRY_ROW;
            } else {
                height += PREVIEW_ROW;
            }
        }
        if self.total == 0 {
            height = 220;
        }
        height.clamp(140, 620)
    }

    fn open_by_default(&self, group: &Group, config: &Config) -> bool {
        !group.always_collapse && group.entries.len() < config.expand_below
    }

    fn headline(&self) -> String {
        if !self.daemon {
            return "mako is not running".to_owned();
        }
        if self.total == 0 {
            return "nothing waiting".to_owned();
        }
        let mut parts = vec![plural(self.total, "notification", "notifications")];
        if self.active > 0 {
            parts.push(format!("{} on screen", self.active));
        }
        if self.critical > 0 {
            parts.push(format!("{} urgent", self.critical));
        }
        parts.join(" · ")
    }

    fn tooltip(&self, config: &Config) -> String {
        if !self.daemon {
            return "mako is not running".to_owned();
        }
        let mut lines = vec![format!(
            "<b>{}</b>",
            crate::format::escape_markup(&self.headline())
        )];
        if self.dnd {
            lines.push("do not disturb".to_owned());
        }
        for group in self.groups.iter().take(4) {
            let Some(latest) = group.entries.first() else {
                continue;
            };
            lines.push(crate::format::escape_markup(&format!(
                "{} · {} — {}",
                group.label,
                group.entries.len(),
                truncate(&summary_of(latest), config.preview_chars.min(64))
            )));
        }
        if self.groups.len() > 4 {
            lines.push(format!("+{} more", self.groups.len() - 4));
        }
        lines.join("\n")
    }

    fn group_model(&self, group: &Group, config: &Config) -> Value {
        let count = group.entries.len();
        let unread = group
            .entries
            .iter()
            .filter(|entry| entry.created_at > self.seen_at)
            .count();
        let critical = group.entries.iter().filter(|entry| entry.is_critical()).count();
        let latest = group.entries.first();

        json!({
            "key": group.key,
            "label": group.label,
            "icon": if group.icon.is_empty() { DEFAULT_ICON } else { &group.icon },
            "count": count,
            "count_label": count.to_string(),
            "unread": unread,
            "has_unread": unread > 0,
            "critical": critical,
            "has_critical": critical > 0,
            "when": latest.map(|entry| relative(entry.created_at, self.now)).unwrap_or_default(),
            "preview_rows": latest
                .map(|entry| vec![truncate(&summary_of(entry), config.preview_chars)])
                .unwrap_or_default(),
            // A short group is worth reading at a glance; a long one is a pile
            // the header already summarises.
            "open_by_default": !group.always_collapse && count < config.expand_below,
            "entries": group
                .entries
                .iter()
                .map(|entry| self.entry_model(entry, config))
                .collect::<Vec<_>>(),
        })
    }

    fn entry_model(&self, entry: &Notification, config: &Config) -> Value {
        let body = clamp_lines(&entry.body, config.body_lines);
        let flattened = preview(&entry.body, config.preview_chars);
        let has_body = !entry.body.trim().is_empty();
        json!({
            "id": entry.id,
            "app": entry.source(),
            "summary": truncate(&entry.summary, config.preview_chars),
            "rows_collapsed": if has_body { vec![flattened.clone()] } else { Vec::new() },
            "rows_expanded": if has_body { vec![body] } else { Vec::new() },
            "when": relative(entry.created_at, self.now),
            "clock": clock(entry.created_at),
            "urgency": entry.urgency,
            "active": entry.active,
            "unread": entry.created_at > self.seen_at,
            // Buttons that only apply to part of the list render as a `for`
            // over a zero- or one-element array: eww has no way to hide a
            // widget without its siblings inheriting the empty allocation.
            "restore_rows": if entry.active { Vec::new() } else { vec!["restore"] },
            "expand_rows": if body_is_clipped(&entry.body, &flattened) { vec!["more"] } else { Vec::new() },
            "progress_rows": if entry.progress >= 0 { vec![entry.progress.clamp(0, 100)] } else { Vec::new() },
            "flip_key": format!("n{}", entry.id),
            "actions": entry.actions
                .iter()
                .map(|action| json!({"key": action.key, "label": action.label}))
                .collect::<Vec<_>>(),
            "has_actions": !entry.actions.is_empty(),
        })
    }
}

fn summary_of(entry: &Notification) -> String {
    match (entry.summary.trim(), entry.body.trim()) {
        ("", body) => body.to_owned(),
        (summary, "") => summary.to_owned(),
        (summary, body) => format!("{summary} — {}", preview(body, 120)),
    }
}

/// Whether folding the body away actually hid anything worth a toggle.
fn body_is_clipped(body: &str, flattened: &str) -> bool {
    body.lines().filter(|line| !line.trim().is_empty()).count() > 1
        || flattened.ends_with('…')
}

