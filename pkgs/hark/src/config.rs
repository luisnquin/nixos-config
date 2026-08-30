use crate::mako::{Field, Notification};
use regex::Regex;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Default)]
pub struct Matcher {
    alternatives: Vec<Vec<(Field, Regex)>>,
}

impl Matcher {
    /// One alternative is an AND over its fields, so `{appName, summary}`
    /// narrows; the list of them is an OR, which is what an app that announces
    /// itself under two different names needs.
    pub fn matches(&self, notification: &Notification) -> bool {
        self.alternatives.iter().any(|fields| {
            fields
                .iter()
                .all(|(field, pattern)| pattern.is_match(notification.field(*field)))
        })
    }

    fn is_empty(&self) -> bool {
        self.alternatives.is_empty()
    }
}

pub struct Rule {
    pub key: String,
    pub label: String,
    pub icon: String,
    pub priority: i64,
    /// Never expand this group on its own, however few notifications it holds.
    pub always_collapse: bool,
    matcher: Matcher,
}

pub struct Config {
    pub rules: Vec<Rule>,
    /// Notifications the centre never accounts for. A daemon is the wrong place
    /// to decide this: an on-screen readout is still a notification, it just has
    /// no life after the popup.
    pub ignore: Matcher,
    pub group_rest_by_app: bool,
    pub expand_below: usize,
    pub preview_chars: usize,
    pub body_lines: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rules: Vec::new(),
            ignore: Matcher::default(),
            group_rest_by_app: true,
            expand_below: 4,
            preview_chars: 160,
            body_lines: 3,
        }
    }
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Config, Box<dyn Error>> {
        let Some(path) = path.or_else(default_path) else {
            return Ok(Config::default());
        };
        let Ok(raw) = fs::read_to_string(&path) else {
            return Ok(Config::default());
        };
        let document: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Config::parse(&document)
    }

    pub fn rule_for(&self, notification: &Notification) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|rule| rule.matcher.matches(notification))
    }

    pub fn ignores(&self, notification: &Notification) -> bool {
        self.ignore.matches(notification)
    }

    fn parse(document: &Value) -> Result<Config, Box<dyn Error>> {
        let default = Config::default();
        let number = |key: &str, fallback: usize| {
            document
                .get(key)
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(fallback)
        };

        let mut rules = Vec::new();
        if let Some(groups) = document.get("groups").and_then(Value::as_object) {
            for (key, group) in groups {
                rules.push(parse_rule(key, group)?);
            }
        }
        // Nix hands over an attribute set, which arrives sorted by name; the
        // declared priority is the only ordering the user actually controls.
        rules.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.key.cmp(&b.key)));

        let ignore = match document.get("ignore") {
            Some(value) => parse_match("ignore", value)?,
            None => Matcher::default(),
        };

        Ok(Config {
            rules,
            ignore,
            group_rest_by_app: document
                .get("groupRestByApp")
                .and_then(Value::as_bool)
                .unwrap_or(default.group_rest_by_app),
            expand_below: number("expandBelow", default.expand_below),
            preview_chars: number("previewChars", default.preview_chars),
            body_lines: number("bodyLines", default.body_lines),
        })
    }
}

/// `context` only names the offender in an error; the shape is the same list of
/// alternatives whether it claims notifications or discards them.
fn parse_match(context: &str, value: &Value) -> Result<Matcher, Box<dyn Error>> {
    let mut alternatives = Vec::new();
    for alternative in value.as_array().into_iter().flatten() {
        let mut fields = Vec::new();
        for (name, pattern) in alternative.as_object().into_iter().flatten() {
            let Some(field) = Field::parse(name) else {
                return Err(format!("{context}: unknown match field {name}").into());
            };
            let Some(pattern) = pattern.as_str() else {
                continue;
            };
            let compiled = Regex::new(pattern)
                .map_err(|error| format!("{context}, field {name}: {error}"))?;
            fields.push((field, compiled));
        }
        if fields.is_empty() {
            return Err(format!("{context}: an alternative with no pattern matches everything").into());
        }
        alternatives.push(fields);
    }
    Ok(Matcher { alternatives })
}

fn parse_rule(key: &str, group: &Value) -> Result<Rule, Box<dyn Error>> {
    let text = |name: &str| group.get(name).and_then(Value::as_str).unwrap_or_default();

    let matcher = parse_match(&format!("group {key}"), group.get("match").unwrap_or(&Value::Null))?;
    if matcher.is_empty() {
        return Err(format!("group {key}: no match patterns, it would never claim anything").into());
    }

    Ok(Rule {
        key: crate::slug(key),
        label: if text("label").is_empty() {
            key.to_owned()
        } else {
            text("label").to_owned()
        },
        icon: text("icon").to_owned(),
        priority: group.get("priority").and_then(Value::as_i64).unwrap_or(50),
        always_collapse: group
            .get("alwaysCollapse")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        matcher,
    })
}

fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("hark/config.json"))
}
