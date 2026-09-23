use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::adb::EMULATOR_BUILD_SERIAL;
use crate::hosts::HostState;
use crate::model::{discovered_id, is_transport_alias, now, Device, Platform, PLACEHOLDER_PREFIX};

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
        self.drop_shared_emulator_keys();

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

    /// Rows keyed by the build serial every emulator shares. Each one merged
    /// however many emulators ran from that image into a single record, with
    /// aliases and a label taken from whichever answered last — so nothing in it
    /// names a device. Discovery mints a real key on the next attach.
    fn drop_shared_emulator_keys(&mut self) {
        let doomed: Vec<String> = self
            .devices
            .iter()
            .filter(|d| d.id.starts_with(EMULATOR_BUILD_SERIAL))
            .map(|d| d.id.clone())
            .collect();

        for id in doomed {
            self.remove(&id);
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

    /// Naming a device in full settles what a substring cannot: `emulator-5554`
    /// is also a substring of `mac/emulator-5554`.
    pub fn find(&self, want: &str) -> Vec<&Device> {
        let exact: Vec<&Device> = self.devices.iter().filter(|d| d.is(want)).collect();

        if !exact.is_empty() {
            return exact;
        }

        self.devices.iter().filter(|d| d.matches(want)).collect()
    }

    /// Drops every row whose id another row already carries as an alias: they
    /// are one device, filed twice because the weaker key was minted before a
    /// transport gave up the stronger one. `keep` holds the ids the caller has
    /// already built a view for, which must not vanish underneath it.
    pub fn fold_aliased(&mut self, keep: &HashSet<String>) {
        let mut i = 0;

        while i < self.devices.len() {
            let id = self.devices[i].id.clone();

            let winner = if keep.contains(&id) {
                None
            } else {
                self.devices
                    .iter()
                    .position(|d| d.id != id && d.aliases.iter().any(|a| a == &id))
            };

            let Some(winner) = winner else {
                i += 1;
                continue;
            };

            let stale = self.devices.remove(i);
            let winner = if winner > i { winner - 1 } else { winner };

            self.devices[winner].absorb(&stale);

            if self.current.as_deref() == Some(stale.id.as_str()) {
                self.current = Some(self.devices[winner].id.clone());
            }
        }
    }

    /// A label equal to the model is a system image many AVDs share, not an AVD
    /// name, so it ties nothing together.
    pub fn fold_wiped(&mut self, live: &HashSet<String>) {
        let mut i = 0;

        while i < self.devices.len() {
            let stale = &self.devices[i];

            let winner = if live.contains(&stale.id) || stale.platform != Platform::Emulator {
                None
            } else {
                self.devices.iter().position(|d| {
                    d.id != stale.id
                        && live.contains(&d.id)
                        && d.platform == Platform::Emulator
                        && d.host == stale.host
                        && d.label == stale.label
                        && d.label != d.model
                })
            };

            let Some(winner) = winner else {
                i += 1;
                continue;
            };

            let mut stale = self.devices.remove(i);
            let winner = if winner > i { winner - 1 } else { winner };

            stale.aliases.retain(|a| !is_transport_alias(a));
            stale.endpoints.clear();

            self.devices[winner].absorb(&stale);

            if self.current.as_deref() == Some(stale.id.as_str()) {
                self.current = Some(self.devices[winner].id.clone());
            }
        }
    }

    /// Takes every transport name this device now answers to off whatever row
    /// used to hold it. Those names are leases, and the device just seen
    /// answering is the one holding it.
    fn evict_stale_leases(&mut self, holder: usize) {
        let claimed: Vec<String> = self.devices[holder]
            .aliases
            .iter()
            .filter(|a| is_transport_alias(a))
            .cloned()
            .collect();

        if claimed.is_empty() {
            return;
        }

        for (i, device) in self.devices.iter_mut().enumerate() {
            if i != holder {
                device.aliases.retain(|a| !claimed.contains(a));
            }
        }
    }

    pub fn upsert(&mut self, device: Device) -> &mut Device {
        let at = match self.devices.iter().position(|d| d.id == device.id) {
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

                i
            }
            None => {
                self.devices.push(device);
                self.devices.len() - 1
            }
        };

        self.evict_stale_leases(at);

        &mut self.devices[at]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Endpoint, Platform};

    fn device(id: &str, aliases: &[&str]) -> Device {
        let mut d = Device::new(id, id, Platform::Android);

        for alias in aliases {
            d.add_alias(*alias);
        }

        d
    }

    #[test]
    fn a_transport_name_follows_the_device_last_seen_answering_to_it() {
        let mut reg = Registry {
            devices: vec![device("android_id:2222bbbb", &["mac/emulator-5554"])],
            ..Default::default()
        };

        let mut booted = device("android_id:3333cccc", &[]);
        booted.add_alias("mac/emulator-5554");

        reg.upsert(booted);

        assert_eq!(reg.devices[0].aliases, Vec::<String>::new());
        assert_eq!(
            reg.by_alias("mac/emulator-5554").unwrap().id,
            "android_id:3333cccc"
        );
    }

    #[test]
    fn a_hardware_id_is_not_a_lease_and_stays_where_it_is() {
        let mut reg = Registry {
            devices: vec![device("SERIALNUMBER01", &["android_id:1111aaaa"])],
            ..Default::default()
        };

        let mut other = device("peer:tailscale:nLhm", &[]);
        other.add_alias("android_id:1111aaaa");

        reg.upsert(other);

        assert_eq!(reg.devices[0].aliases, ["android_id:1111aaaa"]);
    }

    /// Every emulator from one system image answered `ro.serialno` with the same
    /// string, so a row keyed by it is however many emulators in a trench coat.
    #[test]
    fn a_row_keyed_by_the_shared_build_serial_does_not_survive_a_load() {
        let mut reg = Registry {
            devices: vec![
                device(
                    "EMULATOR36X6X11X0",
                    &["emulator-5554", "android_id:2222bbbb"],
                ),
                device("SERIALNUMBER01", &[]),
            ],
            current: Some("EMULATOR36X6X11X0".into()),
            ..Default::default()
        };

        reg.migrate();

        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].id, "SERIALNUMBER01");
        assert_eq!(reg.current, None, "the default target cannot point at it");
    }

    #[test]
    fn a_row_another_row_already_answers_to_is_folded_into_it() {
        let mut reg = Registry {
            devices: vec![
                device("SERIALNUMBER01", &["100.64.0.10:5555", "pixel-9"]),
                device("100.64.0.10:5555", &[]),
            ],
            current: Some("100.64.0.10:5555".into()),
            ..Default::default()
        };

        reg.fold_aliased(&HashSet::new());

        assert_eq!(reg.devices.len(), 1);
        assert_eq!(reg.devices[0].id, "SERIALNUMBER01");
        assert_eq!(
            reg.current.as_deref(),
            Some("SERIALNUMBER01"),
            "the default target follows the row it was folded into"
        );
        assert!(reg.by_alias("100.64.0.10:5555").is_some());
    }

    #[test]
    fn what_the_caller_is_already_holding_a_view_of_stays() {
        let mut reg = Registry {
            devices: vec![
                device("SERIALNUMBER01", &["100.64.0.10:5555"]),
                device("100.64.0.10:5555", &[]),
            ],
            ..Default::default()
        };

        reg.fold_aliased(&HashSet::from(["100.64.0.10:5555".to_string()]));

        assert_eq!(reg.devices.len(), 2);
    }

    #[test]
    fn two_rows_naming_each_other_still_settle_on_one() {
        let mut reg = Registry {
            devices: vec![device("a", &["b"]), device("b", &["a"])],
            ..Default::default()
        };

        reg.fold_aliased(&HashSet::new());

        assert_eq!(reg.devices.len(), 1);
    }

    fn avd(id: &str, host: Option<&str>, name: &str, aliases: &[&str]) -> Device {
        let mut d = device(id, aliases);

        d.label = name.into();
        d.model = "sdk_gphone16k_arm64".into();
        d.platform = Platform::Emulator;
        d.host = host.map(str::to_string);

        d
    }

    fn wiped_twice() -> Registry {
        Registry {
            devices: vec![
                avd("android_id:2222bbbb", Some("mac"), "pixel_7-api36", &[]),
                avd(
                    "android_id:1111aaaa",
                    Some("mac"),
                    "pixel_7-api36",
                    &["mac/emulator-5554"],
                ),
                avd(
                    "android_id:3333cccc",
                    Some("mac"),
                    "pixel_7-api36",
                    &["localhost:5557"],
                ),
                avd(
                    "android_id:4444dddd",
                    Some("mac"),
                    "pixel_7-api36-b",
                    &["mac/emulator-5556"],
                ),
            ],
            current: Some("android_id:2222bbbb".into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_running_avd_takes_over_the_rows_its_wipes_left_behind() {
        let mut reg = wiped_twice();

        reg.fold_wiped(&HashSet::from([
            "android_id:1111aaaa".to_string(),
            "android_id:4444dddd".to_string(),
        ]));

        let ids: Vec<&str> = reg.devices.iter().map(|d| d.id.as_str()).collect();

        assert_eq!(ids, ["android_id:1111aaaa", "android_id:4444dddd"]);
        assert_eq!(reg.current.as_deref(), Some("android_id:1111aaaa"));
        assert_eq!(
            reg.by_alias("android_id:3333cccc").unwrap().id,
            "android_id:1111aaaa",
            "the old id stays resolvable"
        );
        assert!(
            reg.by_alias("localhost:5557").is_none(),
            "a transport the old process answered on is not inherited"
        );
    }

    #[test]
    fn rows_of_an_avd_that_is_not_running_are_left_alone() {
        let mut reg = wiped_twice();

        reg.fold_wiped(&HashSet::from(["android_id:4444dddd".to_string()]));

        assert_eq!(reg.devices.len(), 4);
    }

    #[test]
    fn an_avd_of_the_same_name_on_another_host_is_another_device() {
        let mut reg = Registry {
            devices: vec![
                avd("android_id:1111", Some("mac"), "pixel_7-api36", &[]),
                avd("android_id:2222", None, "pixel_7-api36", &[]),
            ],
            ..Default::default()
        };

        reg.fold_wiped(&HashSet::from(["android_id:2222".to_string()]));

        assert_eq!(reg.devices.len(), 2);
    }

    #[test]
    fn emulators_labelled_by_their_system_image_are_not_folded() {
        let mut reg = Registry {
            devices: vec![
                avd("android_id:1111", Some("mac"), "sdk_gphone16k_arm64", &[]),
                avd("android_id:2222", Some("mac"), "sdk_gphone16k_arm64", &[]),
            ],
            ..Default::default()
        };

        reg.fold_wiped(&HashSet::from(["android_id:2222".to_string()]));

        assert_eq!(reg.devices.len(), 2);
    }

    #[test]
    fn a_handset_is_never_folded_into_an_emulator_of_its_name() {
        let mut handset = device("SERIALNUMBER01", &[]);

        handset.label = "pixel_7-api36".into();
        handset.host = Some("mac".into());

        let mut reg = Registry {
            devices: vec![
                handset,
                avd("android_id:1111aaaa", Some("mac"), "pixel_7-api36", &[]),
            ],
            ..Default::default()
        };

        reg.fold_wiped(&HashSet::from(["android_id:1111aaaa".to_string()]));

        assert_eq!(reg.devices.len(), 2);
    }

    #[test]
    fn endpoints_survive_the_fold() {
        let mut stale = device("100.64.0.10:41939", &[]);
        stale.merge_endpoint(Endpoint::new("100.64.0.10", 41939));

        let mut reg = Registry {
            devices: vec![device("SERIALNUMBER01", &["100.64.0.10:41939"]), stale],
            ..Default::default()
        };

        reg.fold_aliased(&HashSet::new());

        assert_eq!(reg.devices[0].endpoints.len(), 1);
        assert_eq!(reg.devices[0].endpoints[0].port, 41939);
    }
}
