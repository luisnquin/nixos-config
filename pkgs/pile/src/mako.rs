use serde_json::Value;
use std::error::Error;
use std::process::Command;

pub const MAKOCTL: &str = "@makoctl@";

pub struct Notification {
    pub id: u32,
    pub created_at: u64,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub urgency: String,
    pub app_icon: String,
}

pub fn active(id: u32) -> Result<Notification, Box<dyn Error>> {
    let raw = control(&["list", "-j"])?;
    let Value::Array(entries) = serde_json::from_str::<Value>(&raw)? else {
        return Err("makoctl list -j did not return an array".into());
    };
    entries
        .iter()
        .map(parse)
        .find(|notification| notification.id == id)
        .ok_or_else(|| format!("no notification {id} on screen").into())
}

pub fn control(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(MAKOCTL).args(args).output()?;
    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr);
        return Err(format!("makoctl {}: {}", args.join(" "), reason.trim()).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn parse(entry: &Value) -> Notification {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };

    Notification {
        id: entry.get("id").and_then(Value::as_u64).unwrap_or(0) as u32,
        created_at: entry.get("created_at").and_then(Value::as_u64).unwrap_or(0),
        app_name: text("app_name"),
        summary: text("summary"),
        body: text("body"),
        urgency: text("urgency"),
        app_icon: text("app_icon"),
    }
}
