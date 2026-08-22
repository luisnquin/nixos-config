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
    #[serde(default)]
    pub reap_when_idle: bool,
    #[serde(default)]
    pub requires: Option<Requires>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Requires {
    pub any_file_exists: Vec<String>,
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
        let config: Self = serde_json::from_str(&contents).map_err(LoadError::Parse)?;
        for (name, server) in &config.servers {
            let Some(requires) = &server.requires else {
                continue;
            };
            if server.scope == Scope::Global {
                return Err(LoadError::Invalid(format!(
                    "server {name}: a requires precondition needs workspace scope; \
                     a global instance has no workspace to resolve it against"
                )));
            }
            if requires.any_file_exists.is_empty() {
                return Err(LoadError::Invalid(format!(
                    "server {name}: requires.any_file_exists lists no files"
                )));
            }
        }
        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cannot read gateway config: {0}")]
    Read(std::io::Error),
    #[error("cannot parse gateway config: {0}")]
    Parse(serde_json::Error),
    #[error("invalid gateway config: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(config: &str) -> Result<Config, LoadError> {
        let directory = std::env::temp_dir().join(format!(
            "nyx-mcp-gateway-config-test-{}-{config:p}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create scratch directory");
        let path = directory.join("config.json");
        std::fs::write(&path, config).expect("write config");
        let loaded = Config::load(&path);
        let _ = std::fs::remove_dir_all(&directory);
        loaded
    }

    #[test]
    fn requires_on_a_global_server_is_refused() {
        let error = load(
            r#"{"servers": {"x": {"command": "x", "scope": "global",
                "requires": {"any_file_exists": ["encore.app"]}}}}"#,
        )
        .expect_err("global requires must not load");
        assert!(error.to_string().contains("workspace scope"), "{error}");
    }

    #[test]
    fn requires_without_files_is_refused() {
        let error = load(
            r#"{"servers": {"x": {"command": "x", "scope": "workspace",
                "requires": {"any_file_exists": []}}}}"#,
        )
        .expect_err("empty requires must not load");
        assert!(error.to_string().contains("no files"), "{error}");
    }

    #[test]
    fn requires_on_a_workspace_server_loads() {
        let config = load(
            r#"{"servers": {"x": {"command": "x", "scope": "workspace",
                "requires": {"any_file_exists": ["encore.app"]}}}}"#,
        )
        .expect("valid config");
        assert!(config.servers["x"].requires.is_some());
    }
}
