use std::fmt;

use serde::{Deserialize, Serialize};

pub type Unix = i64;

/// Key prefix for a peer named by discovery that has never answered over adb,
/// and so has no hardware id to be filed under yet.
pub const PLACEHOLDER_PREFIX: &str = "peer:";

pub const EMULATOR_SERIAL_PREFIX: &str = "emulator-";

/// Whether a name says where a device answered rather than which device it is.
/// An adb serial, a `host/serial` pair and a `host:port` are all leases: the
/// port an emulator frees is handed to the next one to boot, so a row may hold
/// such a name only until another device is seen answering to it.
pub fn is_transport_alias(name: &str) -> bool {
    let name = name.rsplit('/').next().unwrap_or(name);

    if name.starts_with(EMULATOR_SERIAL_PREFIX) {
        return true;
    }

    name.rsplit_once(':')
        .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok())
}

/// The source is part of the key so two sources naming the same machine do not
/// collide.
pub fn discovered_id(source: &str, key: &str) -> String {
    format!("{source}:{key}")
}

pub fn now() -> Unix {
    chrono::Utc::now().timestamp()
}

pub fn ago(t: Unix) -> String {
    let d = (now() - t).max(0);

    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", d / 60),
        3600..=86399 => format!("{}h ago", d / 3600),
        _ => format!("{}d ago", d / 86400),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Android,
    Emulator,
    Ios,
    Simulator,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Android => "android",
            Platform::Emulator => "emu",
            Platform::Ios => "ios",
            Platform::Simulator => "sim",
        }
    }

    pub fn is_adb(self) -> bool {
        matches!(self, Platform::Android | Platform::Emulator)
    }

    /// Driven by running commands on its host rather than over a transport this
    /// machine can hold open: CoreSimulator has no socket to forward.
    pub fn is_hosted(self) -> bool {
        matches!(self, Platform::Ios | Platform::Simulator)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How deterministic a wireless port is. `adb tcpip` survives until the device
/// reboots and the persistent property survives that too; anything else is a
/// port Android rerolls on every wireless-debugging toggle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pin {
    #[default]
    None,
    Session,
    Persistent,
}

impl Pin {
    pub fn as_str(self) -> &'static str {
        match self {
            Pin::None => "",
            Pin::Session => "pin",
            Pin::Persistent => "pin!",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub pin: Pin,
    #[serde(default)]
    pub last_ok: Option<Unix>,
}

impl Endpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            pin: Pin::None,
            last_ok: None,
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// A device as remembered across runs. `id` is a hardware-level identifier, not
/// an adb serial: the same handset is `R58N70ABCDE` over USB and a host:port
/// over the network, so keying on the serial would mean a new entry per
/// transport change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub model: String,
    pub platform: Platform,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Stable key from whichever source first named this device, kept resolvable
    /// after a transport reveals the real hardware id and supersedes it.
    #[serde(default)]
    pub discovered_id: Option<String>,
    /// The ssh host this device hangs off. Only a name — where it resolves and
    /// how it is reached is ssh's business.
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<Endpoint>,
    #[serde(default)]
    pub last_connected: Option<Unix>,
}

impl Device {
    pub fn new(id: impl Into<String>, label: impl Into<String>, platform: Platform) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            model: String::new(),
            platform,
            aliases: Vec::new(),
            discovered_id: None,
            host: None,
            endpoints: Vec::new(),
            last_connected: None,
        }
    }

    /// An exact hit on one of the names this device is filed under. Substring
    /// matching is what makes a target quick to type, and also what makes
    /// `emulator-5554` name `rose/emulator-5554` as well; typing one in full has
    /// to settle that.
    pub fn is(&self, want: &str) -> bool {
        let eq = |s: &str| s.eq_ignore_ascii_case(want);

        eq(&self.id) || eq(&self.label) || self.aliases.iter().any(|a| eq(a))
    }

    pub fn matches(&self, want: &str) -> bool {
        let want = want.to_lowercase();

        let hit = |s: &str| s.to_lowercase().contains(&want);

        hit(&self.id)
            || hit(&self.label)
            || hit(&self.model)
            || hit(self.platform.as_str())
            || self.aliases.iter().any(|a| hit(a))
            || self.host.as_deref().is_some_and(hit)
            || self.endpoints.iter().any(|e| hit(&e.addr()))
    }

    /// Every address this device has been reached at, best first, which is also
    /// the list of hosts worth scanning when no remembered port answers.
    pub fn hosts(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();

        for e in self.ranked_endpoints() {
            if !out.contains(&e.host) {
                out.push(e.host.clone());
            }
        }

        out
    }

    pub fn add_alias(&mut self, alias: impl Into<String>) {
        let alias = alias.into();

        if !alias.is_empty() && alias != self.id && !self.aliases.contains(&alias) {
            self.aliases.push(alias);
        }
    }

    /// Pinned ports never move, so they outrank ephemeral ones however recently
    /// those worked.
    pub fn ranked_endpoints(&self) -> Vec<&Endpoint> {
        let mut out: Vec<&Endpoint> = self.endpoints.iter().collect();

        out.sort_by_key(|e| {
            let rank = match e.pin {
                Pin::Persistent => 0,
                Pin::Session => 1,
                Pin::None => 2,
            };

            (rank, -e.last_ok.unwrap_or(0))
        });

        out
    }

    pub fn record_endpoint(&mut self, host: &str, port: u16, pin: Pin) {
        self.merge_endpoint(Endpoint {
            host: host.to_string(),
            port,
            pin,
            last_ok: Some(now()),
        });
    }

    /// `last_ok` only ever moves forward, and only from a caller that saw the
    /// port answer: discovery guesses at `:5555` constantly, and stamping those
    /// would make a guess outrank the port that really worked.
    pub fn merge_endpoint(&mut self, incoming: Endpoint) {
        if let Some(e) = self
            .endpoints
            .iter_mut()
            .find(|e| e.host == incoming.host && e.port == incoming.port)
        {
            if incoming.pin != Pin::None {
                e.pin = incoming.pin;
            }

            if incoming.last_ok > e.last_ok {
                e.last_ok = incoming.last_ok;
            }

            return;
        }

        self.endpoints.push(incoming);

        // else the list grows once per wireless-debugging toggle, forever
        if self.endpoints.len() > 8 {
            let ranked: Vec<String> = self
                .ranked_endpoints()
                .into_iter()
                .take(8)
                .map(|e| e.addr())
                .collect();

            self.endpoints.retain(|e| ranked.contains(&e.addr()));
        }
    }

    /// Folds a placeholder record into this device, keeping its key resolvable
    /// so `phone device connect peer:…` and any saved default still land here.
    pub fn absorb(&mut self, other: &Device) {
        if self.label.is_empty() {
            self.label = other.label.clone();
        }

        if self.model.is_empty() {
            self.model = other.model.clone();
        }

        if self.discovered_id.is_none() {
            self.discovered_id = other.discovered_id.clone();
        }

        if self.host.is_none() {
            self.host = other.host.clone();
        }

        self.add_alias(other.id.clone());

        for alias in &other.aliases {
            self.add_alias(alias.clone());
        }

        for e in &other.endpoints {
            self.merge_endpoint(e.clone());
        }

        if other.last_connected > self.last_connected {
            self.last_connected = other.last_connected;
        }
    }
}

/// Runtime reachability, recomputed on every survey and never persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reach {
    Attached {
        serial: String,
        wireless: bool,
    },
    Unauthorized {
        serial: String,
    },
    Online,
    /// Defined on its host and not running. Unlike `Known`, which is only a
    /// memory of something once seen, this was read off the host just now: it
    /// is there, and it can be started.
    Off,
    Offline {
        last_seen: Option<Unix>,
    },
    Known,
}

impl Reach {
    pub fn serial(&self) -> Option<&str> {
        match self {
            Reach::Attached { serial, .. } | Reach::Unauthorized { serial } => Some(serial),
            _ => None,
        }
    }

    pub fn is_attached(&self) -> bool {
        matches!(self, Reach::Attached { .. })
    }

    pub fn label(&self) -> String {
        match self {
            // an emulator's transport is neither of the two a handset can have
            Reach::Attached { serial, wireless } => match () {
                _ if serial.starts_with(EMULATOR_SERIAL_PREFIX) => "attached/emu".into(),
                _ if *wireless => "attached/tcp".into(),
                _ => "attached/usb".into(),
            },
            Reach::Unauthorized { .. } => "unauthorized".into(),
            Reach::Online => "online".into(),
            Reach::Off => "off".into(),
            Reach::Offline { last_seen } => match last_seen {
                Some(t) => format!("offline {}", ago(*t)),
                None => "offline".into(),
            },
            Reach::Known => "known".into(),
        }
    }

    /// Picks the default target and sorts the picker.
    pub fn rank(&self) -> u8 {
        match self {
            Reach::Attached { .. } => 0,
            Reach::Online => 1,
            Reach::Unauthorized { .. } => 2,
            Reach::Off => 3,
            Reach::Known => 4,
            Reach::Offline { .. } => 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct View {
    pub device: Device,
    pub reach: Reach,
    /// A serial is only unique within one adb server, so every command aimed at
    /// this device has to go back through the one that saw it.
    pub server: crate::adb::Server,
}

impl View {
    pub fn new(device: Device, reach: Reach) -> Self {
        Self {
            device,
            reach,
            server: crate::adb::Server::Local,
        }
    }

    pub fn on(mut self, server: crate::adb::Server) -> Self {
        self.server = server;
        self
    }

    /// `Online` means a discovery source still lists the device, and adb cannot
    /// screencap through that. Hosted platforms capture on their host, so being
    /// listed there is all the reachability they get.
    pub fn can_shoot(&self) -> bool {
        if self.device.platform.is_hosted() {
            return self.reach == Reach::Online;
        }

        self.reach.is_attached()
    }

    /// A queued shot sits until a transport appears, which for a device that was
    /// never connected is never; without a reason that is indistinguishable from
    /// the program ignoring the keypress.
    pub fn blocked(&self) -> Option<Blocked> {
        if self.can_shoot() {
            return None;
        }

        Some(match self.reach {
            Reach::Unauthorized { .. } => Blocked::Unauthorized,
            Reach::Online => Blocked::Disconnected,
            _ => Blocked::Away,
        })
    }
}

/// Kept apart from its wording so the phrasing, which names keys the user has to
/// press, stays with the UI that owns them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    Disconnected,
    Unauthorized,
    Away,
}

#[cfg(test)]
mod tests {

    /// A device that is off is on its host right now and can be started; a
    /// known one is only a memory of something once seen. Sorting them the
    /// other way round would offer the worse of the two first.
    #[test]
    fn off_outranks_what_is_only_remembered() {
        assert!(Reach::Online.rank() < Reach::Off.rank());
        assert!(Reach::Off.rank() < Reach::Known.rank());
        assert!(Reach::Known.rank() < Reach::Offline { last_seen: None }.rank());
        assert_eq!(Reach::Off.label(), "off");
    }
    use super::*;

    #[test]
    fn a_shot_that_can_run_is_waiting_for_nothing() {
        let attached = View::new(
            Device::new("phone", "phone", Platform::Android),
            Reach::Attached {
                serial: "39askdj2".into(),
                wireless: false,
            },
        );

        let listed = View::new(
            Device::new("phone", "phone", Platform::Android),
            Reach::Online,
        );

        let sim = View::new(Device::new("udid", "udid", Platform::Ios), Reach::Online);

        assert_eq!(attached.blocked(), None);
        assert_eq!(sim.blocked(), None, "a hosted device shoots from its host");
        assert_eq!(listed.blocked(), Some(Blocked::Disconnected));
    }

    #[test]
    fn naming_a_device_in_full_beats_being_a_substring_of_another() {
        let local = Device::new("emulator-5554", "nyx-remote-android", Platform::Emulator);
        let hosted = Device::new(
            "rose/emulator-5554",
            "nyx-remote-android",
            Platform::Emulator,
        );

        assert!(hosted.matches("emulator-5554"), "substrings still match");

        assert!(local.is("emulator-5554"));
        assert!(!hosted.is("emulator-5554"));
        assert!(
            hosted.is("ROSE/emulator-5554"),
            "ids are not case sensitive"
        );
    }

    #[test]
    fn tells_where_a_device_answered_apart_from_which_device_it_is() {
        assert!(is_transport_alias("emulator-5554"));
        assert!(is_transport_alias("rose/emulator-5554"));
        assert!(is_transport_alias("100.127.25.101:41939"));
        assert!(is_transport_alias("faraday:5555"));

        assert!(!is_transport_alias("58281FDCG001K5"));
        assert!(!is_transport_alias("android_id:29a1ed706918672b"));
        assert!(!is_transport_alias("rose/00008101-000C601611D2001E"));
        assert!(!is_transport_alias("peer:tailscale:ncBC9WsKTg11CNTRL"));
    }

    #[test]
    fn an_emulator_transport_is_neither_usb_nor_tcp() {
        let emu = Reach::Attached {
            serial: "emulator-5554".into(),
            wireless: false,
        };
        let usb = Reach::Attached {
            serial: "39askdj2".into(),
            wireless: false,
        };
        let tcp = Reach::Attached {
            serial: "10.0.0.4:5555".into(),
            wireless: true,
        };

        assert_eq!(emu.label(), "attached/emu");
        assert_eq!(usb.label(), "attached/usb");
        assert_eq!(tcp.label(), "attached/tcp");
    }
}
