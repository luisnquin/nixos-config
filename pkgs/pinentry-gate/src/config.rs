use std::path::Path;

use serde::Deserialize;

pub const CONFIG_PATH: &str = "/etc/pinentry-gate/config.json";

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub vt: Option<u16>,
    #[serde(default)]
    pub terminal: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    300
}

impl Default for Config {
    fn default() -> Self {
        Config {
            user: String::new(),
            vt: None,
            terminal: Vec::new(),
            timeout: default_timeout(),
        }
    }
}

impl Config {
    pub fn load() -> Config {
        let path = std::env::var("PINENTRY_GATE_CONFIG").unwrap_or_else(|_| CONFIG_PATH.to_string());
        Config::from_path(Path::new(&path))
    }

    pub fn from_path(path: &Path) -> Config {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_their_defaults() {
        let config: Config = serde_json::from_str(r#"{"user":"me"}"#).unwrap();
        assert_eq!(config.user, "me");
        assert_eq!(config.vt, None);
        assert!(config.terminal.is_empty());
        assert_eq!(config.timeout, 300);
    }

    #[test]
    fn a_missing_file_is_the_default() {
        let config = Config::from_path(Path::new("/nonexistent/pinentry-gate.json"));
        assert_eq!(config.timeout, 300);
    }
}
