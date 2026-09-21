use std::fs;
use std::io;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::machine::{Millis, Power};

const INPUT_EVENT_SIZE: usize = 24;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const MAX_DRAIN_READS: usize = 64;

fn clock_ms(clock: libc::clockid_t) -> Millis {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(clock, &mut ts) };
    (ts.tv_sec as Millis) * 1_000 + (ts.tv_nsec as Millis) / 1_000_000
}

pub fn boottime_ms() -> Millis {
    clock_ms(libc::CLOCK_BOOTTIME)
}

pub fn monotonic_ms() -> Millis {
    clock_ms(libc::CLOCK_MONOTONIC)
}

pub struct SleepDetector {
    offset: Millis,
}

impl SleepDetector {
    pub fn new() -> Self {
        Self {
            offset: boottime_ms().saturating_sub(monotonic_ms()),
        }
    }

    pub fn observe(&mut self, boottime: Millis, monotonic: Millis, threshold: Millis) -> bool {
        let offset = boottime.saturating_sub(monotonic);
        let grew = offset.saturating_sub(self.offset);
        self.offset = offset;
        grew >= threshold
    }

    pub fn sample(&mut self, threshold: Millis) -> bool {
        self.observe(boottime_ms(), monotonic_ms(), threshold)
    }
}

impl Default for SleepDetector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn power(root: &Path) -> Power {
    let Ok(entries) = fs::read_dir(root) else {
        return Power::Unknown;
    };

    let mut saw_mains = false;
    let mut unreadable = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(kind) = fs::read_to_string(path.join("type")) else {
            unreadable = true;
            continue;
        };
        if kind.trim() != "Mains" {
            continue;
        }
        saw_mains = true;
        match fs::read_to_string(path.join("online")) {
            Ok(text) => match text.trim() {
                "1" => return Power::Mains,
                "0" => {}
                _ => unreadable = true,
            },
            Err(_) => unreadable = true,
        }
    }

    if saw_mains && !unreadable {
        Power::Battery
    } else {
        Power::Unknown
    }
}

pub fn online(targets: &[String], timeout: Duration) -> bool {
    if targets.is_empty() {
        return false;
    }

    let (tx, rx) = mpsc::channel();
    for target in targets {
        let target = target.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let _ = tx.send(probe(&target, timeout));
        });
    }
    drop(tx);

    let deadline = Instant::now() + timeout + Duration::from_millis(250);
    loop {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            return false;
        };
        match rx.recv_timeout(left) {
            Ok(true) => return true,
            Ok(false) => continue,
            Err(_) => return false,
        }
    }
}

pub struct Prober {
    request: mpsc::Sender<()>,
    results: mpsc::Receiver<bool>,
    inflight: bool,
}

impl Prober {
    pub fn new(targets: Vec<String>, timeout: Duration) -> Self {
        let (request, orders) = mpsc::channel::<()>();
        let (answers, results) = mpsc::channel::<bool>();
        thread::spawn(move || {
            while orders.recv().is_ok() {
                if answers.send(online(&targets, timeout)).is_err() {
                    return;
                }
            }
        });
        Self {
            request,
            results,
            inflight: false,
        }
    }

    pub fn request(&mut self) {
        if self.inflight {
            return;
        }
        if self.request.send(()).is_ok() {
            self.inflight = true;
        }
    }

    pub fn inflight(&self) -> bool {
        self.inflight
    }

    pub fn take(&mut self) -> Option<bool> {
        match self.results.try_recv() {
            Ok(verdict) => {
                self.inflight = false;
                Some(verdict)
            }
            Err(_) => None,
        }
    }
}

fn probe(target: &str, timeout: Duration) -> bool {
    if let Ok(addr) = target.parse::<SocketAddr>() {
        return TcpStream::connect_timeout(&addr, timeout).is_ok();
    }

    let Ok(addrs) = target.to_socket_addrs() else {
        return false;
    };
    addrs
        .take(3)
        .any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok())
}

struct Device {
    path: PathBuf,
    fd: OwnedFd,
}

pub struct InputWatcher {
    root: PathBuf,
    ignore: Vec<String>,
    devices: Vec<Device>,
    last_input: Millis,
}

impl InputWatcher {
    pub fn new(root: impl Into<PathBuf>, ignore: Vec<String>, now: Millis) -> Self {
        let mut watcher = Self {
            root: root.into(),
            ignore: ignore.into_iter().map(|s| s.to_lowercase()).collect(),
            devices: Vec::new(),
            last_input: now,
        };
        watcher.rescan();
        watcher
    }

    pub fn last_input(&self) -> Millis {
        self.last_input
    }

    pub fn idle_since(&self) -> Option<Millis> {
        if self.devices.is_empty() {
            None
        } else {
            Some(self.last_input)
        }
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn fds(&self) -> Vec<RawFd> {
        self.devices.iter().map(|d| d.fd.as_raw_fd()).collect()
    }

    pub fn rescan(&mut self) {
        self.devices.retain(|d| d.path.exists());

        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.starts_with("event") || self.devices.iter().any(|d| d.path == path) {
                continue;
            }
            if self.ignored(name) {
                continue;
            }
            if let Ok(fd) = open_nonblocking(&path) {
                self.devices.push(Device { path, fd });
            }
        }
    }

    fn ignored(&self, event_name: &str) -> bool {
        if self.ignore.is_empty() {
            return false;
        }
        let label = fs::read_to_string(format!("/sys/class/input/{event_name}/device/name"))
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        !label.is_empty() && self.ignore.iter().any(|pat| label.contains(pat))
    }

    pub fn drain(&mut self, now: Millis) -> bool {
        let mut buf = [0u8; INPUT_EVENT_SIZE * 32];
        let mut touched = false;

        for device in &self.devices {
            for _ in 0..MAX_DRAIN_READS {
                let n = unsafe {
                    libc::read(
                        device.fd.as_raw_fd(),
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                    )
                };
                if n <= 0 {
                    break;
                }
                touched |= has_human_input(&buf[..n as usize]);
                if (n as usize) < buf.len() {
                    break;
                }
            }
        }

        if touched {
            self.last_input = now;
        }
        touched
    }
}

fn open_nonblocking(path: &Path) -> io::Result<OwnedFd> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn counts_as_input(ev_type: u16) -> bool {
    matches!(ev_type, EV_KEY | EV_REL | EV_ABS)
}

fn has_human_input(buf: &[u8]) -> bool {
    let (events, _partial) = buf.as_chunks::<INPUT_EVENT_SIZE>();
    events.iter().any(|ev| {
        let ev_type = u16::from_ne_bytes([ev[16], ev[17]]);
        counts_as_input(ev_type)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(ev_type: u16) -> [u8; INPUT_EVENT_SIZE] {
        let mut ev = [0u8; INPUT_EVENT_SIZE];
        ev[16..18].copy_from_slice(&ev_type.to_ne_bytes());
        ev
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("outage-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn supply(root: &Path, name: &str, kind: &str, online: Option<&str>) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("type"), format!("{kind}\n")).unwrap();
        if let Some(value) = online {
            fs::write(dir.join("online"), format!("{value}\n")).unwrap();
        }
    }

    #[test]
    fn mains_online_reads_as_mains() {
        let root = scratch("power-ac");
        supply(&root, "AC", "Mains", Some("1"));
        supply(&root, "BAT1", "Battery", None);
        assert_eq!(power(&root), Power::Mains);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn mains_offline_reads_as_battery() {
        let root = scratch("power-bat");
        supply(&root, "AC", "Mains", Some("0"));
        supply(&root, "BAT1", "Battery", None);
        assert_eq!(power(&root), Power::Battery);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_second_mains_still_on_wins() {
        let root = scratch("power-dual");
        supply(&root, "AC", "Mains", Some("0"));
        supply(&root, "USBC", "Mains", Some("1"));
        assert_eq!(power(&root), Power::Mains);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_mains_supply_with_no_online_file_is_unknown_not_battery() {
        let root = scratch("power-noonline");
        supply(&root, "AC", "Mains", None);
        assert_eq!(
            power(&root),
            Power::Unknown,
            "a mains supply we cannot read is not proof of a blackout"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_malformed_online_value_is_unknown_not_battery() {
        let root = scratch("power-garbage");
        supply(&root, "AC", "Mains", Some("unknown"));
        assert_eq!(power(&root), Power::Unknown);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_unreadable_online_file_is_unknown_even_beside_a_working_one() {
        let root = scratch("power-mixed");
        supply(&root, "AC", "Mains", Some("0"));
        supply(&root, "USBC", "Mains", None);
        assert_eq!(
            power(&root),
            Power::Unknown,
            "one readable 'off' does not cover for a mains supply we cannot see"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_absent_or_empty_power_tree_is_unknown() {
        assert_eq!(
            power(Path::new("/nonexistent/power_supply")),
            Power::Unknown
        );
        let root = scratch("power-empty");
        assert_eq!(
            power(&root),
            Power::Unknown,
            "no mains supply at all is not evidence of anything"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn only_key_pointer_and_touch_events_reset_idle() {
        assert!(counts_as_input(EV_KEY));
        assert!(counts_as_input(EV_REL));
        assert!(counts_as_input(EV_ABS));
        assert!(!counts_as_input(0x00));
        assert!(!counts_as_input(0x04));
        assert!(!counts_as_input(0x05));
    }

    #[test]
    fn a_buffer_of_switch_events_is_not_human_input() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&event(0x05));
        buf.extend_from_slice(&event(0x00));
        assert!(!has_human_input(&buf));

        buf.extend_from_slice(&event(EV_KEY));
        assert!(has_human_input(&buf));
    }

    #[test]
    fn a_partial_trailing_event_is_ignored() {
        let mut buf = event(EV_KEY).to_vec();
        buf.truncate(INPUT_EVENT_SIZE - 1);
        assert!(!has_human_input(&buf));
    }

    #[test]
    fn a_boottime_jump_with_a_frozen_monotonic_reads_as_a_sleep() {
        let mut detector = SleepDetector { offset: 0 };
        assert!(detector.observe(600_000, 0, 5_000));
        assert!(!detector.observe(660_000, 60_000, 5_000));
    }

    #[test]
    fn a_short_offset_drift_is_not_a_sleep() {
        let mut detector = SleepDetector { offset: 0 };
        assert!(!detector.observe(1_000, 0, 5_000));
    }

    #[test]
    fn no_probe_targets_is_reported_offline() {
        assert!(!online(&[], Duration::from_millis(10)));
    }

    #[test]
    fn an_unroutable_target_is_reported_offline() {
        assert!(!online(
            &["192.0.2.1:443".to_string()],
            Duration::from_millis(200)
        ));
    }

    #[test]
    fn an_input_watcher_over_an_empty_tree_finds_nothing() {
        let root = scratch("input-empty");
        let mut watcher = InputWatcher::new(&root, Vec::new(), 1_000);
        assert_eq!(watcher.device_count(), 0);
        assert!(!watcher.drain(2_000));
        assert_eq!(watcher.last_input(), 1_000);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_watcher_with_no_devices_reports_idle_as_unobservable() {
        let root = scratch("input-none");
        let watcher = InputWatcher::new(&root, Vec::new(), 1_000);
        assert_eq!(
            watcher.idle_since(),
            None,
            "an unwatched keyboard must never age into proven idle"
        );
        assert_eq!(watcher.last_input(), 1_000);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_watcher_with_a_device_reports_idle_normally() {
        let root = scratch("input-one");
        let path = root.join("event0");
        let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);

        let watcher = InputWatcher::new(&root, Vec::new(), 5_000);
        assert_eq!(watcher.device_count(), 1);
        assert_eq!(watcher.idle_since(), Some(5_000));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_prober_answers_off_the_caller_thread() {
        let mut prober = Prober::new(
            vec!["192.0.2.1:443".to_string()],
            Duration::from_millis(150),
        );
        assert!(prober.take().is_none(), "nothing was asked for yet");
        prober.request();
        assert!(prober.inflight());
        prober.request();

        let deadline = Instant::now() + Duration::from_secs(5);
        let verdict = loop {
            if let Some(verdict) = prober.take() {
                break verdict;
            }
            assert!(Instant::now() < deadline, "the prober never answered");
            thread::sleep(Duration::from_millis(10));
        };
        assert!(!verdict, "TEST-NET-1 is never routable");
        assert!(!prober.inflight());
    }
}
