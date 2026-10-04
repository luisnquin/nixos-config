use chrono::{DateTime, Local};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

pub fn relative(then_ms: u64, now: u64) -> String {
    if then_ms == 0 {
        return "unknown".to_owned();
    }
    let seconds = now.saturating_sub(then_ms) / 1000;
    match seconds {
        0..=44 => "just now".to_owned(),
        45..=5399 => format!("{}m ago", (seconds + 30) / 60),
        5400..=86399 => format!("{}h ago", (seconds + 1800) / 3600),
        _ => format!("{}d ago", (seconds + 43200) / 86400),
    }
}

pub fn clock(then_ms: u64) -> String {
    if then_ms == 0 {
        return String::new();
    }
    let Some(utc) = DateTime::from_timestamp((then_ms / 1000) as i64, 0) else {
        return String::new();
    };
    utc.with_timezone(&Local).format("%H:%M").to_string()
}

/// A body collapsed onto one line, for the row that is not expanded.
pub fn preview(body: &str, limit: usize) -> String {
    let flattened = body.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&flattened, limit)
}

pub fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let head: String = text.chars().take(limit.saturating_sub(1)).collect();
    format!("{}…", head.trim_end())
}

/// The body as the expanded row shows it: at most `lines` lines, blanks gone.
pub fn clamp_lines(body: &str, lines: usize) -> String {
    let kept: Vec<&str> = body
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .take(lines)
        .collect();
    kept.join("\n")
}

pub fn escape_markup(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

pub fn plural(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}
