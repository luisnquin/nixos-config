use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::machine::Thresholds;

pub const DEFAULT_CONFIG: &str = "/etc/outage/config.json";
pub const DEFAULT_SOCKET: &str = "/run/outage/control.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub user: String,
    pub control_group: String,
    pub socket: PathBuf,
    pub power_supply_root: PathBuf,
    pub input_root: PathBuf,
    pub ignore_input_devices: Vec<String>,
    pub probes: Vec<String>,
    pub probe_timeout_secs: u64,
    pub offline_grace_secs: u64,
    pub idle_grace_secs: u64,
    pub sleep_interval_secs: u64,
    pub network_window_secs: u64,
    pub interaction_grace_secs: u64,
    pub terminate_grace_secs: u64,
    pub ignore_inhibitors: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user: String::new(),
            control_group: "wheel".to_string(),
            socket: PathBuf::from(DEFAULT_SOCKET),
            power_supply_root: PathBuf::from("/sys/class/power_supply"),
            input_root: PathBuf::from("/dev/input"),
            ignore_input_devices: Vec::new(),
            probes: vec![
                "1.1.1.1:443".to_string(),
                "8.8.8.8:443".to_string(),
                "one.one.one.one:443".to_string(),
            ],
            probe_timeout_secs: 3,
            offline_grace_secs: 120,
            idle_grace_secs: 300,
            sleep_interval_secs: 600,
            network_window_secs: 60,
            interaction_grace_secs: 300,
            terminate_grace_secs: 20,
            ignore_inhibitors: true,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err),
        }
    }

    pub fn thresholds(&self) -> Thresholds {
        Thresholds {
            offline_grace: Duration::from_secs(self.offline_grace_secs),
            idle_grace: Duration::from_secs(self.idle_grace_secs),
            sleep_interval: Duration::from_secs(self.sleep_interval_secs),
            network_window: Duration::from_secs(self.network_window_secs),
            interaction_grace: Duration::from_secs(self.interaction_grace_secs),
            terminate_grace: Duration::from_secs(self.terminate_grace_secs),
        }
    }

    pub fn probe_timeout(&self) -> Duration {
        Duration::from_secs(self.probe_timeout_secs)
    }

    pub fn control_timeout(&self) -> Duration {
        Duration::from_secs(self.terminate_grace_secs * 2 + 15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_config_falls_back_to_defaults() {
        let cfg = Config::load(Path::new("/nonexistent/outage.json")).unwrap();
        assert_eq!(cfg.socket, PathBuf::from(DEFAULT_SOCKET));
        assert_eq!(cfg.thresholds(), Thresholds::default());
    }

    #[test]
    fn a_partial_config_keeps_the_remaining_defaults() {
        let cfg: Config = serde_json::from_str(r#"{"user":"ori","idle_grace_secs":42}"#).unwrap();
        assert_eq!(cfg.user, "ori");
        assert_eq!(cfg.thresholds().idle_grace, Duration::from_secs(42));
        assert_eq!(
            cfg.thresholds().offline_grace,
            Thresholds::default().offline_grace
        );
    }

    #[test]
    fn the_client_outwaits_the_longest_teardown_it_can_be_queued_behind() {
        for grace in [5, 20, 120] {
            let cfg = Config {
                terminate_grace_secs: grace,
                ..Config::default()
            };
            let worst = Duration::from_secs(grace + 2 * 5 + 1);
            assert!(
                cfg.control_timeout() > worst,
                "grace {grace}s: {:?} does not cover {:?}",
                cfg.control_timeout(),
                worst
            );
        }
    }

    #[test]
    fn an_unknown_key_is_rejected_rather_than_silently_dropped() {
        let err = serde_json::from_str::<Config>(r#"{"idle_grace_seconds":42}"#);
        assert!(err.is_err());
    }

    #[test]
    fn the_defaults_round_trip_through_json() {
        let text = serde_json::to_string(&Config::default()).unwrap();
        let parsed: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.thresholds(), Config::default().thresholds());
        assert_eq!(parsed.probes, Config::default().probes);
    }
}
