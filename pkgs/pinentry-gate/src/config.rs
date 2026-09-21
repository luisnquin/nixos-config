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
    #[serde(default)]
    pub sound: SoundConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SoundConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_timeout() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

fn default_volume() -> f32 {
    0.4
}

impl Default for Config {
    fn default() -> Self {
        Config {
            user: String::new(),
            vt: None,
            terminal: Vec::new(),
            timeout: default_timeout(),
            sound: SoundConfig::default(),
        }
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        SoundConfig {
            enable: true,
            device: None,
            volume: default_volume(),
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
        assert!(config.sound.enable);
        assert_eq!(config.sound.device, None);
    }

    #[test]
    fn a_null_sound_device_is_none() {
        let config: Config = serde_json::from_str(r#"{"sound":{"enable":false,"device":null,"volume":0.2}}"#).unwrap();
        assert!(!config.sound.enable);
        assert_eq!(config.sound.device, None);
        assert_eq!(config.sound.volume, 0.2);
    }

    #[test]
    fn a_missing_file_is_the_default() {
        let config = Config::from_path(Path::new("/nonexistent/pinentry-gate.json"));
        assert_eq!(config.timeout, 300);
    }
}
