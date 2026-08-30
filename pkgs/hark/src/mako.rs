use serde_json::Value;
use std::error::Error;
use std::process::Command;

pub const MAKOCTL: &str = "@makoctl@";

pub const DND_MODE: &str = "do-not-disturb";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    AppName,
    Summary,
    Body,
    Category,
    DesktopEntry,
    Tag,
    Urgency,
}

impl Field {
    pub fn parse(name: &str) -> Option<Field> {
        Some(match name {
            "appName" => Field::AppName,
            "summary" => Field::Summary,
            "body" => Field::Body,
            "category" => Field::Category,
            "desktopEntry" => Field::DesktopEntry,
            "tag" => Field::Tag,
            "urgency" => Field::Urgency,
            _ => return None,
        })
    }
}

pub struct Action {
    pub key: String,
    pub label: String,
}

pub struct Notification {
    pub id: u32,
    pub created_at: u64,
    pub app_name: String,
    pub category: String,
    pub desktop_entry: String,
    pub summary: String,
    pub body: String,
    pub tag: String,
    pub urgency: String,
    pub progress: i64,
    pub actions: Vec<Action>,
    pub active: bool,
}

impl Notification {
    pub fn field(&self, field: Field) -> &str {
        match field {
            Field::AppName => &self.app_name,
            Field::Summary => &self.summary,
            Field::Body => &self.body,
            Field::Category => &self.category,
            Field::DesktopEntry => &self.desktop_entry,
            Field::Tag => &self.tag,
            Field::Urgency => &self.urgency,
        }
    }

    /// What the app calls itself, falling back to anything that identifies it.
    pub fn source(&self) -> &str {
        for candidate in [&self.app_name, &self.desktop_entry, &self.category] {
            if !candidate.is_empty() {
                return candidate;
            }
        }
        "unknown"
    }

    pub fn is_critical(&self) -> bool {
        self.urgency == "critical" || self.urgency == "high"
    }
}

/// mako's two lists: what is on screen right now, and what has expired into the
/// history ring. An id lives in exactly one of them, so they concatenate.
pub fn collect() -> Result<Vec<Notification>, Box<dyn Error>> {
    let mut notifications = read_list(&["list", "-j"], true)?;
    notifications.extend(read_list(&["history", "-j"], false)?);
    notifications.sort_by_key(|notification| std::cmp::Reverse(notification.created_at));
    Ok(notifications)
}

pub fn running() -> bool {
    control(&["mode"]).is_ok()
}

pub fn modes() -> Vec<String> {
    control(&["mode"])
        .map(|output| output.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

pub fn control(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(MAKOCTL).args(args).output()?;
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr);
        return Err(format!("makoctl {}: {}", args.join(" "), reason.trim()).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn read_list(args: &[&str], active: bool) -> Result<Vec<Notification>, Box<dyn Error>> {
    let Ok(raw) = control(args) else {
        return Ok(Vec::new());
    };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&raw) else {
        return Ok(Vec::new());
    };
    Ok(entries
        .iter()
        .map(|entry| parse(entry, active))
        .collect())
}

fn parse(entry: &Value, active: bool) -> Notification {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };

    let actions = entry
        .get("actions")
        .and_then(Value::as_object)
        .map(|actions| {
            actions
                .iter()
                .map(|(key, label)| Action {
                    key: key.clone(),
                    label: label.as_str().unwrap_or(key).to_owned(),
                })
                .collect()
        })
        .unwrap_or_default();

    Notification {
        id: entry.get("id").and_then(Value::as_u64).unwrap_or(0) as u32,
        created_at: entry.get("created_at").and_then(Value::as_u64).unwrap_or(0),
        app_name: text("app_name"),
        category: text("category"),
        desktop_entry: text("desktop_entry"),
        summary: text("summary"),
        body: text("body"),
        tag: text("tag"),
        urgency: text("urgency"),
        progress: entry.get("progress").and_then(Value::as_i64).unwrap_or(-1),
        actions,
        active,
    }
}
