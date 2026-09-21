use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use crate::config::Config;
use crate::control::{self, Reply, Request};
use crate::driver::{Driver, World};
use crate::effects::{SystemEffects, WakeAlarm};
use crate::machine::{Machine, Millis, Power};
use crate::runner::Runner;
use crate::sense::{self, InputWatcher, Prober, SleepDetector};

const SLEEP_THRESHOLD_MS: Millis = 5_000;

const MIN_POLL_MS: Millis = 100;
const MAX_POLL_MS: Millis = 10_000;

pub fn run(config: Config) -> io::Result<()> {
    let uid = control::user_id(&config.user).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no such user: '{}'", config.user),
        )
    })?;

    let alarm = WakeAlarm::new().map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("CLOCK_BOOTTIME_ALARM timerfd unavailable ({err}); CAP_WAKE_ALARM is required"),
        )
    })?;
    let alarm_fd = alarm.as_raw_fd();

    let listener = control::bind(&config.socket, &config.control_group)?;
    listener.set_nonblocking(true)?;

    let watcher = InputWatcher::new(
        &config.input_root,
        config.ignore_input_devices.clone(),
        sense::boottime_ms(),
    );
    crate::log(&format!(
        "watching {} input device(s) for idle, user {} (uid {uid})",
        watcher.device_count(),
        config.user
    ));

    let mut world = PolledWorld {
        listener,
        power_root: config.power_supply_root.clone(),
        watcher,
        prober: Prober::new(config.probes.clone(), config.probe_timeout()),
        verdict: None,
        awaiting: Vec::new(),
        resume: None,
    };

    let effects = SystemEffects::new(uid, config.ignore_inhibitors, alarm);
    let mut runner = Runner::new(Machine::new(config.thresholds()), effects);
    let mut driver = Driver::new(
        sense::boottime_ms(),
        config.probe_timeout() * 2 + Duration::from_secs(5),
    );
    let mut detector = SleepDetector::new();

    loop {
        let wait_ms = driver.step(&mut world, &mut runner);

        let (alarm_ready, _) = world.block(alarm_fd, wait_ms)?;

        let slept = detector.sample(SLEEP_THRESHOLD_MS);
        let fired = alarm_ready && runner.effects().alarm().take_expiration();
        if slept || fired {
            world.note_resume(fired);
        }
    }
}

struct PolledWorld {
    listener: UnixListener,
    power_root: PathBuf,
    watcher: InputWatcher,
    prober: Prober,
    verdict: Option<(Millis, bool)>,
    awaiting: Vec<UnixStream>,
    resume: Option<bool>,
}

impl PolledWorld {
    fn block(&mut self, alarm_fd: RawFd, wait_ms: Millis) -> io::Result<(bool, bool)> {
        let input_fds = self.watcher.fds();
        let mut fds = Vec::with_capacity(input_fds.len() + 2);
        fds.push(pollfd(self.listener.as_raw_fd()));
        fds.push(pollfd(alarm_fd));
        fds.extend(input_fds.iter().map(|fd| pollfd(*fd)));

        let timeout = wait_ms.clamp(MIN_POLL_MS, MAX_POLL_MS) as libc::c_int;
        let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
        if rc < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok((false, false));
            }
            return Err(err);
        }

        let ready = |slot: &libc::pollfd| slot.revents != 0;
        Ok((ready(&fds[1]), fds[2..].iter().any(ready)))
    }

    fn note_resume(&mut self, scheduled: bool) {
        self.resume = Some(self.resume.unwrap_or(false) || scheduled);
    }

    fn reply_to(stream: &UnixStream, reply: &Reply) {
        if let Err(err) = control::write_reply(stream, reply) {
            crate::log(&format!("control reply failed: {err}"));
        }
    }
}

impl World for PolledWorld {
    fn take_requests(&mut self) -> Vec<Request> {
        let mut requests = Vec::new();
        loop {
            let stream = match self.listener.accept() {
                Ok((stream, _)) => stream,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    crate::log(&format!("control accept failed: {err}"));
                    break;
                }
            };

            match control::read_request(&stream) {
                Ok(request) => {
                    requests.push(request);
                    self.awaiting.push(stream);
                }
                Err(err) => Self::reply_to(
                    &stream,
                    &Reply::new(false, "unknown", format!("bad request: {err}")),
                ),
            }
        }
        requests
    }

    fn answer(&mut self, replies: Vec<Reply>) {
        for (stream, reply) in self.awaiting.drain(..).zip(replies.iter()) {
            Self::reply_to(&stream, reply);
        }
    }

    fn take_resume(&mut self) -> Option<bool> {
        self.resume.take()
    }

    fn refresh(&mut self) {
        self.watcher.drain(sense::boottime_ms());
        if let Some(online) = self.prober.take() {
            self.verdict = Some((sense::boottime_ms(), online));
        }
    }

    fn idle_since(&self) -> Option<Millis> {
        self.watcher.idle_since()
    }

    fn power(&mut self) -> Power {
        sense::power(&self.power_root)
    }

    fn connectivity(&mut self) -> Option<(Millis, bool)> {
        if let Some(online) = self.prober.take() {
            self.verdict = Some((sense::boottime_ms(), online));
        }
        self.verdict
    }

    fn request_probe(&mut self) {
        self.prober.request();
    }

    fn probe_inflight(&self) -> bool {
        self.prober.inflight()
    }

    fn rescan_input(&mut self) {
        self.watcher.rescan();
    }

    fn awake_ms(&self) -> Millis {
        sense::monotonic_ms()
    }
}

fn pollfd(fd: RawFd) -> libc::pollfd {
    libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    }
}
