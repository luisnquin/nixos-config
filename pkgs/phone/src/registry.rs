use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::hosts::HostState;
use crate::model::{discovered_id, now, Device, PLACEHOLDER_PREFIX};

/// The id prefix placeholders carried before discovery sources were named.
const LEGACY_PLACEHOLDER_PREFIX: &str = "tailnet:";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub devices: Vec<Device>,
    #[serde(default)]
    pub current: Option<String>,
    /// Which ssh hosts to survey, and each tunnel's local port.
    #[serde(default)]
    pub hosts: Vec<HostState>,

    #[serde(skip)]
    path: PathBuf,
}

pub fn state_dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("phone")
}

impl Registry {
    pub fn load() -> Result<Self> {
        Self::load_from(&state_dir().join("devices.json"))
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let mut reg = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<Registry>(&bytes)
                .with_context(|| format!("parsing {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Registry::default(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };

        reg.path = path.to_path_buf();
        reg.migrate();

        Ok(reg)
    }

    /// Rewrites placeholder keys minted before the id carried its source. Only
    /// the id is worth migrating — discovery rebuilds the rest, and a stale id
    /// shows the same device twice.
    fn migrate(&mut self) {
        let mut renamed: Vec<(String, String)> = Vec::new();

        for device in &mut self.devices {
            let Some(node_id) = device.id.strip_prefix(LEGACY_PLACEHOLDER_PREFIX) else {
                continue;
            };

            let discovered = discovered_id("tailscale", node_id);
            let id = format!("{PLACEHOLDER_PREFIX}{discovered}");

            renamed.push((device.id.clone(), id.clone()));

            device.discovered_id = Some(discovered);
            device.id = id;
        }

        for (old, new) in renamed {
            if self.current.as_deref() == Some(old.as_str()) {
                self.current = Some(new);
            }
        }
    }

    /// A rename rather than a truncate-and-fill: two `phone` invocations racing
    /// on this file is normal, and a reader must not catch it half-done.
    pub fn save(&self) -> Result<()> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }

        let dir = self
            .path
            .parent()
            .context("registry path has no parent directory")?;

        std::fs::create_dir_all(dir)?;

        let tmp = dir.join(format!("devices.json.{}.tmp", std::process::id()));
        let body = serde_json::to_vec_pretty(self)?;

        std::fs::write(&tmp, &body)?;
        std::fs::rename(&tmp, &self.path)?;

        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Device> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    pub fn by_alias(&self, alias: &str) -> Option<&Device> {
        self.devices
            .iter()
            .find(|d| d.id == alias || d.aliases.iter().any(|a| a == alias))
    }

    pub fn by_discovered_id(&self, discovered: &str) -> Option<&Device> {
        self.devices
            .iter()
            .find(|d| d.discovered_id.as_deref() == Some(discovered))
    }

    /// A discovered-only placeholder holding `host`, other than `keep`. Once a
    /// transport answers there it is the same handset under a weaker key.
    pub fn placeholder_at(&self, host: &str, keep: &str) -> Option<&Device> {
        self.devices.iter().find(|d| {
            d.id != keep
                && d.id.starts_with(PLACEHOLDER_PREFIX)
                && d.endpoints.iter().any(|e| e.host == host)
        })
    }

    pub fn find(&self, want: &str) -> Vec<&Device> {
        self.devices.iter().filter(|d| d.matches(want)).collect()
    }

    pub fn upsert(&mut self, device: Device) -> &mut Device {
        match self.devices.iter().position(|d| d.id == device.id) {
            Some(i) => {
                let existing = &mut self.devices[i];

                // discovery knows less than a completed connect, so it must
                // not blank what it did not observe
                existing.label = device.label;
                existing.platform = device.platform;

                if !device.model.is_empty() {
                    existing.model = device.model;
                }

                if device.discovered_id.is_some() {
                    existing.discovered_id = device.discovered_id;
                }

                if device.host.is_some() {
                    existing.host = device.host;
                }

                for alias in device.aliases {
                    existing.add_alias(alias);
                }

                for e in device.endpoints {
                    existing.merge_endpoint(e);
                }

                if device.last_connected > existing.last_connected {
                    existing.last_connected = device.last_connected;
                }

                &mut self.devices[i]
            }
            None => {
                self.devices.push(device);
                self.devices.last_mut().expect("just pushed")
            }
        }
    }

    pub fn touch(&mut self, id: &str) {
        if let Some(d) = self.get_mut(id) {
            d.last_connected = Some(now());
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.devices.len();

        self.devices.retain(|d| d.id != id);

        if self.current.as_deref() == Some(id) {
            self.current = None;
        }

        self.devices.len() != before
    }

    pub fn host_mut(&mut self, name: &str) -> &mut HostState {
        match self.hosts.iter().position(|h| h.name == name) {
            Some(i) => &mut self.hosts[i],
            None => {
                self.hosts.push(HostState::new(name));
                self.hosts.last_mut().expect("just pushed")
            }
        }
    }

    /// Opt-in per host: an ssh config lists machines that have nothing to do
    /// with phones, and every enabled one costs a round trip per refresh.
    pub fn enabled_hosts(&self) -> Vec<&HostState> {
        self.hosts.iter().filter(|h| h.enabled).collect()
    }

    /// An alias that left the config takes its state with it, so a renamed host
    /// does not sit disabled forever pointing at nothing. An enabled one is kept
    /// either way — it answered a probe once, which beats a stanza as evidence.
    pub fn sync_hosts(&mut self, found: &[String]) {
        self.hosts
            .retain(|h| h.enabled || found.iter().any(|f| f == &h.name));

        for name in found {
            if !self.hosts.iter().any(|h| &h.name == name) {
                self.hosts.push(HostState::new(name.clone()));
            }
        }

        self.hosts.sort_by_key(|h| {
            found
                .iter()
                .position(|n| n == &h.name)
                .unwrap_or(usize::MAX)
        });
    }
}
