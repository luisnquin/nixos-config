use std::fmt;

use serde::{Deserialize, Serialize};

pub type Unix = i64;

/// Key prefix for a peer some discovery source named but that has never
/// answered over adb, and so has no hardware id to be filed under yet.
pub const PLACEHOLDER_PREFIX: &str = "peer:";

/// How adb names an emulator: the prefix plus the console port it answers on.
pub const EMULATOR_SERIAL_PREFIX: &str = "emulator-";

/// Builds the id a discovery source hands a device it found but never spoke to.
/// The source is part of the key so two sources naming the same machine do not
/// collide, and so nothing downstream has to know which source it came from.
pub fn discovered_id(source: &str, key: &str) -> String {
    format!("{source}:{key}")
}

pub fn now() -> Unix {
    chrono::Utc::now().timestamp()
}

/// Compact "2h ago" rendering; the exact instant is never interesting here, only
/// whether an endpoint is worth trying again.
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

    /// Whether the platform is driven by running commands on its host rather
    /// than by a transport this machine can hold open. CoreSimulator has no
    /// socket to forward, so a simulator is only ever reachable as a shell on
    /// the mac that owns it.
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
/// reboots; the persistent property survives that too but needs root. Anything
/// else is an ephemeral port that Android rerolls on every wireless-debugging
/// toggle, so it is only worth one speculative retry.
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
/// an adb serial: the same handset is `R58N70ABCDE` over USB and
/// `100.127.25.101:37419` over a routed network, so keying on the adb serial
/// would mean a new history entry on every transport change.
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
    /// after a transport reveals the real hardware id and supersedes it. Carries
    /// its source as a prefix so nothing downstream special-cases one.
    #[serde(default)]
    pub discovered_id: Option<String>,
    /// The ssh host this device hangs off, when it is not on this machine. Only
    /// a name — where it resolves and how it is reached is ssh's business.
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

    /// Every address this device has ever been reached at, best first. Discovery
    /// sources record what they know as endpoints, so this is also the list of
    /// hosts worth scanning when no remembered port answers.
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

    /// Endpoints worth dialling, best first: pinned ports never move, so they go
    /// ahead of ephemeral ones regardless of how recently those worked.
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

    /// Records an endpoint that has just answered.
    pub fn record_endpoint(&mut self, host: &str, port: u16, pin: Pin) {
        self.merge_endpoint(Endpoint {
            host: host.to_string(),
            port,
            pin,
            last_ok: Some(now()),
        });
    }

    /// Merges what is known about an endpoint. `last_ok` only ever moves
    /// forward, and only from a caller that actually saw the port answer:
    /// discovery guesses at `:5555` constantly, and stamping those would make
    /// an untested guess outrank the port that really worked.
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

        // an ephemeral port that has been superseded is dead weight; keep the
        // list from growing once per wireless-debugging toggle, forever.
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

    /// Folds a placeholder record into this device. A discovered-only entry is
    /// keyed on whatever its source calls it, because nothing has ever spoken
    /// adb to it; the first attach reveals the hardware id and supersedes it.
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

        // the old key stays resolvable, so `phone connect peer:…` and any saved
        // default still land on the right device.
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
    Attached { serial: String, wireless: bool },
    Unauthorized { serial: String },
    Online,
    Offline { last_seen: Option<Unix> },
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
            // an emulator's transport is neither of the two a handset can have,
            // and "usb" is the one reading that is never true of it.
            Reach::Attached { serial, wireless } => match () {
                _ if serial.starts_with(EMULATOR_SERIAL_PREFIX) => "attached/emu".into(),
                _ if *wireless => "attached/tcp".into(),
                _ => "attached/usb".into(),
            },
            Reach::Unauthorized { .. } => "unauthorized".into(),
            Reach::Online => "online".into(),
            Reach::Offline { last_seen } => match last_seen {
                Some(t) => format!("offline {}", ago(*t)),
                None => "offline".into(),
            },
            Reach::Known => "known".into(),
        }
    }

    /// Ordering used to pick a default target and to sort the picker.
    pub fn rank(&self) -> u8 {
        match self {
            Reach::Attached { .. } => 0,
            Reach::Online => 1,
            Reach::Unauthorized { .. } => 2,
            Reach::Known => 3,
            Reach::Offline { .. } => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct View {
    pub device: Device,
    pub reach: Reach,
    /// Which adb server saw it. A serial is only unique within one server, so
    /// every command aimed at this device has to go back through the same one.
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

    /// A capture needs a live transport, not just an answer: `Online` means a
    /// discovery source still lists the device, and adb cannot screencap
    /// through that. Hosted platforms are the exception — their capture runs on
    /// the host, so being listed there is all the reachability they ever get.
    pub fn can_shoot(&self) -> bool {
        if self.device.platform.is_hosted() {
            return self.reach == Reach::Online;
        }

        self.reach.is_attached()
    }

    /// What `can_shoot` is waiting for, or `None` when it is not waiting.
    ///
    /// A queued shot is silent by nature — it sits until a transport appears,
    /// which for a device that was never connected is never. Without a reason
    /// the wait is indistinguishable from the program ignoring the keypress.
    pub fn blocked(&self) -> Option<Blocked> {
        if self.can_shoot() {
            return None;
        }

        Some(match self.reach {
            Reach::Unauthorized { .. } => Blocked::Unauthorized,
            // listed by a discovery source, which is an answer and not a
            // transport: adb cannot screencap over it.
            Reach::Online => Blocked::Disconnected,
            _ => Blocked::Away,
        })
    }
}

/// Why a capture cannot run yet. Kept apart from its wording so the phrasing,
/// which names keys the user has to press, stays with the UI that owns them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    Disconnected,
    Unauthorized,
    Away,
}

#[cfg(test)]
mod tests {
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
