use chrono::{DateTime, Local};

pub fn clock(then_ms: u64) -> String {
    if then_ms == 0 {
        return String::new();
    }
    let Some(utc) = DateTime::from_timestamp((then_ms / 1000) as i64, 0) else {
        return String::new();
    };
    utc.with_timezone(&Local).format("%H:%M").to_string()
}

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
