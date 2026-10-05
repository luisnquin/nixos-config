use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};

use crate::lease;
use crate::model::{Device, Platform};

pub const TTL: Duration = Duration::from_secs(20 * 60);

static LOADED: OnceLock<Config> = OnceLock::new();

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub lease: Lease,
    #[serde(default)]
    pub hosts: BTreeMap<String, Host>,
    #[serde(default)]
    pub pools: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub devices: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    #[serde(default, deserialize_with = "span")]
    pub ttl: Option<Duration>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    #[serde(default)]
    pub clone: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(default)]
    pub kind: Option<Kind>,
    #[serde(default)]
    pub pick: Option<Pick>,
    #[serde(default)]
    pub lease: Lease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Physical,
    Virtual,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pick {
    First,
    #[default]
    Normal,
    Last,
    Never,
}

fn span<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
    let text = String::deserialize(d)?;

    crate::cli::parse_duration(&text)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

pub fn path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("phone/config.toml")
}

pub fn load() -> Result<()> {
    let path = path();
    let config = match std::fs::read_to_string(&path) {
        Ok(text) => Config::parse(&text).with_context(|| format!("in {}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };

    let _ = LOADED.set(config);

    Ok(())
}

pub fn get() -> &'static Config {
    LOADED.get_or_init(Config::default)
}

impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        Ok(toml::from_str(text)?)
    }

    pub fn entry(&self, device: &Device) -> Option<&Entry> {
        let name = name(device);

        self.devices
            .iter()
            .find(|(want, _)| **want == name || device.is(want))
            .map(|(_, entry)| entry)
    }

    pub fn ttl(&self, device: &Device) -> Duration {
        self.entry(device)
            .and_then(|e| e.lease.ttl)
            .or(self.lease.ttl)
            .unwrap_or(TTL)
    }

    pub fn pick(&self, device: &Device) -> Pick {
        match self.entry(device).and_then(|e| e.pick) {
            Some(pick) => pick,
            None if self.physical(device) => Pick::Last,
            None => Pick::Normal,
        }
    }

    pub fn physical(&self, device: &Device) -> bool {
        match self.entry(device).and_then(|e| e.kind) {
            Some(kind) => kind == Kind::Physical,
            None => matches!(device.platform, Platform::Android | Platform::Ios),
        }
    }

    pub fn clones(&self, host: Option<&str>) -> bool {
        host.and_then(|h| self.hosts.get(h)).is_some_and(|h| h.clone)
    }

    pub fn pooled(&self, device: &Device) -> bool {
        match self.pools.get(device.platform.os()) {
            Some(pool) => {
                let name = name(device);

                pool.iter().any(|want| *want == name || device.is(want))
            }
            None => true,
        }
    }
}

pub fn name(device: &Device) -> String {
    let id = lease::key(device).id;

    match id.strip_prefix(crate::model::AVD_PREFIX) {
        Some(avd) => avd.to_string(),
        None => device.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::tests::avd;

    const OWNER: &str = r#"
[lease]
ttl = "20m"

[hosts.rose]
clone = false

[pools]
android = ["pixel_7-api36", "pixel_7-api36-b", "pixel_7-api36-c"]

[devices.faraday]
kind = "physical"
pick = "last"

[devices.faraday.lease]
ttl = "2m"
"#;

    #[test]
    fn a_missing_file_is_every_default() {
        let c = Config::default();
        let pixel = avd("pixel_7-api36");

        assert_eq!(c.ttl(&pixel), TTL);
        assert_eq!(c.pick(&pixel), Pick::Normal);
        assert!(c.pooled(&pixel));
        assert!(!c.clones(Some("rose")));
        assert!(!c.physical(&pixel));
    }

    #[test]
    fn the_owner_file_reads_as_written() {
        let c = Config::parse(OWNER).unwrap();
        let faraday = Device::new("R58N", "faraday", Platform::Android);

        assert_eq!(c.ttl(&faraday), Duration::from_secs(120));
        assert_eq!(c.ttl(&avd("pixel_7-api36")), TTL);
        assert_eq!(c.pick(&faraday), Pick::Last);
        assert!(c.physical(&faraday));
        assert!(c.pooled(&avd("pixel_7-api36-b")));
        assert!(!c.pooled(&avd("tablet-api34")));
        assert!(!c.pooled(&faraday));
        assert!(!c.clones(Some("rose")));
    }

    #[test]
    fn a_typo_is_refused_rather_than_ignored() {
        assert!(Config::parse("[lease]\nttl = \"2 fortnights\"").is_err());
        assert!(Config::parse("[devices.x]\npick = \"sometimes\"").is_err());
        assert!(Config::parse("[hosts.rose]\nclones = true").is_err());
    }

    #[test]
    fn a_handset_is_a_last_resort_unless_the_config_says_otherwise() {
        let handset = Device::new("R58N", "faraday", Platform::Android);
        let bare = Config::default();

        assert_eq!(bare.pick(&handset), Pick::Last);

        let virtualized = Config::parse("[devices.faraday]\nkind = \"virtual\"").unwrap();
        let normal = Config::parse("[devices.faraday]\npick = \"normal\"").unwrap();

        assert_eq!(virtualized.pick(&handset), Pick::Normal);
        assert_eq!(normal.pick(&handset), Pick::Normal);
    }
}
