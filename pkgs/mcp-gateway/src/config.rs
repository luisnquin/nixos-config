use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub servers: HashMap<String, Server>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub credentials: HashMap<String, PathBuf>,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_idle")]
    pub idle_seconds: u64,
    #[serde(default)]
    pub notifications: NotificationPolicy,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPolicy {
    #[default]
    Drop,
    Broadcast,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    Global,
    Workspace,
}

fn default_timeout() -> u64 {
    120
}

fn default_idle() -> u64 {
    300
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        let contents = std::fs::read_to_string(path).map_err(LoadError::Read)?;
        serde_json::from_str(&contents).map_err(LoadError::Parse)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cannot read gateway config: {0}")]
    Read(std::io::Error),
    #[error("cannot parse gateway config: {0}")]
    Parse(serde_json::Error),
}
