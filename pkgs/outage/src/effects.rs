use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::machine::Millis;
use crate::sense;

const LOGINCTL: &str = "@loginctl@";
const SYSTEMCTL: &str = "@systemctl@";

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const COMMAND_POLL: Duration = Duration::from_millis(25);
const CGROUP_POLL: Duration = Duration::from_millis(250);

pub trait Effects {
    fn now(&self) -> Millis;
    fn terminate_user_session(&mut self, grace: Duration) -> io::Result<()>;
    fn arm_wake(&mut self, after: Duration) -> io::Result<()>;
    fn cancel_wake(&mut self) -> io::Result<()>;
    fn suspend(&mut self) -> io::Result<()>;
}

pub struct WakeAlarm {
    fd: OwnedFd,
}

impl WakeAlarm {
    pub fn new() -> io::Result<Self> {
        let fd = unsafe {
            libc::timerfd_create(
                libc::CLOCK_BOOTTIME_ALARM,
                libc::TFD_CLOEXEC | libc::TFD_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn arm(&self, after: Duration) -> io::Result<()> {
        self.set(after)
    }

    pub fn cancel(&self) -> io::Result<()> {
        self.set(Duration::ZERO)
    }

    fn set(&self, after: Duration) -> io::Result<()> {
        let spec = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: after.as_secs() as libc::time_t,
                tv_nsec: after.subsec_nanos() as _,
            },
        };
        let rc =
            unsafe { libc::timerfd_settime(self.fd.as_raw_fd(), 0, &spec, std::ptr::null_mut()) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn take_expiration(&self) -> bool {
        let mut buf = [0u8; 8];
        let n = unsafe {
            libc::read(
                self.fd.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        n == buf.len() as isize && u64::from_ne_bytes(buf) > 0
    }
}

pub struct SystemEffects {
    uid: u32,
    ignore_inhibitors: bool,
    alarm: WakeAlarm,
    cgroup_root: PathBuf,
    proc_root: PathBuf,
}

impl SystemEffects {
    pub fn new(uid: u32, ignore_inhibitors: bool, alarm: WakeAlarm) -> Self {
        Self {
            uid,
            ignore_inhibitors,
            alarm,
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
            proc_root: PathBuf::from("/proc"),
        }
    }

    pub fn alarm(&self) -> &WakeAlarm {
        &self.alarm
    }

    fn slice(&self) -> String {
        format!("user-{}.slice", self.uid)
    }

    fn slice_path(&self) -> PathBuf {
        self.cgroup_root.join("user.slice").join(self.slice())
    }

    fn survivors(&self) -> io::Result<Vec<i32>> {
        collect_cgroup_pids(&self.slice_path())
    }

    fn wait_until_empty(&self, until: Instant) -> io::Result<Vec<i32>> {
        loop {
            let left = self.survivors()?;
            if left.is_empty() {
                return Ok(left);
            }
            let now = Instant::now();
            if now >= until {
                return Ok(left);
            }
            thread::sleep(CGROUP_POLL.min(until - now));
        }
    }
}

impl Effects for SystemEffects {
    fn now(&self) -> Millis {
        sense::boottime_ms()
    }

    fn terminate_user_session(&mut self, grace: Duration) -> io::Result<()> {
        let uid = self.uid.to_string();
        let slice = self.slice();
        let started = Instant::now();
        let deadline = started + grace;
        let reserved = (grace / 4).clamp(Duration::from_secs(2), Duration::from_secs(8));
        let graceful_deadline = deadline.checked_sub(reserved).unwrap_or(started);

        let mut problems: Vec<String> = Vec::new();
        let mut attempt = |program: &str, args: &[&str], until: Instant| {
            let budget = until
                .saturating_duration_since(Instant::now())
                .clamp(Duration::from_millis(500), COMMAND_TIMEOUT);
            if let Err(err) = run_bounded(program, args, budget) {
                crate::log(&err.to_string());
                problems.push(err.to_string());
            }
        };

        attempt(LOGINCTL, &["terminate-user", &uid], graceful_deadline);
        attempt(
            SYSTEMCTL,
            &["stop", "--no-block", &format!("user@{uid}.service")],
            graceful_deadline,
        );

        if self.wait_until_empty(graceful_deadline)?.is_empty() {
            crate::log(&format!(
                "user {uid}: session gone gracefully, {slice} verified empty"
            ));
            return Ok(());
        }

        attempt(
            SYSTEMCTL,
            &["kill", "--kill-whom=all", "--signal=SIGKILL", &slice],
            deadline,
        );
        attempt(SYSTEMCTL, &["stop", &slice], deadline);

        let left = self.wait_until_empty(deadline)?;
        if !left.is_empty() {
            let mut killed = 0usize;
            for pid in &left {
                match kill_in_slice(&self.proc_root, *pid, self.uid, &slice) {
                    Killed::Signalled | Killed::Gone => killed += 1,
                    Killed::Skipped => {}
                    Killed::Failed(err) => problems.push(format!("kill {pid}: {err}")),
                }
            }
            crate::log(&format!(
                "user {uid}: {} process(es) outlived {slice}, {killed} signalled directly",
                left.len()
            ));
        }

        let left = self.wait_until_empty(deadline.max(Instant::now() + CGROUP_POLL))?;
        if !left.is_empty() {
            return Err(io::Error::other(format!(
                "user {uid}: {} process(es) still charged to {slice} after {:.1}s{}",
                left.len(),
                started.elapsed().as_secs_f32(),
                context(&problems)
            )));
        }

        crate::log(&format!(
            "user {uid}: {slice} verified empty after {:.1}s",
            started.elapsed().as_secs_f32()
        ));
        Ok(())
    }

    fn arm_wake(&mut self, after: Duration) -> io::Result<()> {
        self.alarm.arm(after)
    }

    fn cancel_wake(&mut self) -> io::Result<()> {
        self.alarm.cancel()
    }

    fn suspend(&mut self) -> io::Result<()> {
        let mut args = vec!["suspend"];
        if self.ignore_inhibitors {
            args.push("-i");
        }
        run_bounded(SYSTEMCTL, &args, COMMAND_TIMEOUT)
    }
}

fn context(problems: &[String]) -> String {
    if problems.is_empty() {
        String::new()
    } else {
        format!(" ({})", problems.join("; "))
    }
}

fn run_bounded(program: &str, args: &[&str], timeout: Duration) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|err| io::Error::new(err.kind(), format!("{program} {args:?}: {err}")))?;

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(status) if status.success() => return Ok(()),
            Some(status) => {
                return Err(io::Error::other(format!(
                    "{program} {args:?} exited {status}"
                )))
            }
            None => {}
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{program} {args:?} exceeded {timeout:?}"),
            ));
        }
        thread::sleep(COMMAND_POLL);
    }
}

#[derive(Debug)]
enum Killed {
    Signalled,
    Gone,
    Skipped,
    Failed(io::Error),
}

fn kill_in_slice(proc_root: &Path, pid: i32, uid: u32, slice: &str) -> Killed {
    // Bind the signal to a pidfd before checking /proc, so PID reuse cannot redirect the kill.
    match pidfd_open(pid) {
        Ok(fd) => {
            if !in_cgroup(proc_root, pid, slice) || proc_uid(proc_root, pid) != Some(uid) {
                return Killed::Skipped;
            }
            match pidfd_kill(&fd) {
                Ok(()) => Killed::Signalled,
                Err(err) if err.raw_os_error() == Some(libc::ESRCH) => Killed::Gone,
                Err(err) => Killed::Failed(err),
            }
        }
        Err(err) if err.raw_os_error() == Some(libc::ESRCH) => Killed::Gone,
        Err(err) => Killed::Failed(err),
    }
}

fn pidfd_open(pid: i32) -> io::Result<OwnedFd> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
}

fn pidfd_kill(fd: &OwnedFd) -> io::Result<()> {
    let rc = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            libc::SIGKILL,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn in_cgroup(proc_root: &Path, pid: i32, slice: &str) -> bool {
    fs::read_to_string(proc_root.join(pid.to_string()).join("cgroup"))
        .is_ok_and(|text| text.lines().any(|line| line.contains(slice)))
}

fn proc_uid(proc_root: &Path, pid: i32) -> Option<u32> {
    let text = fs::read_to_string(proc_root.join(pid.to_string()).join("status")).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|rest| rest.split_whitespace().next()?.parse().ok())
}

pub fn collect_cgroup_pids(root: &Path) -> io::Result<Vec<i32>> {
    let mut pids = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let procs = dir.join("cgroup.procs");
        match fs::read_to_string(&procs) {
            Ok(text) => pids.extend(
                text.lines()
                    .filter_map(|line| line.trim().parse::<i32>().ok()),
            ),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(annotate(&procs, err)),
        }

        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(annotate(&dir, err)),
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(annotate(&dir, err)),
            };
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                stack.push(entry.path());
            }
        }
    }

    Ok(pids)
}

fn annotate(path: &Path, err: io::Error) -> io::Error {
    io::Error::new(err.kind(), format!("{}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("outage-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn cgroup_walk_gathers_pids_from_the_whole_subtree() {
        let root = scratch("cg-walk");
        let nested = root.join("app.slice").join("compositor.scope");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("cgroup.procs"), "10\n11\n").unwrap();
        fs::write(nested.join("cgroup.procs"), "12\n").unwrap();

        let mut pids = collect_cgroup_pids(&root).unwrap();
        pids.sort_unstable();
        assert_eq!(pids, vec![10, 11, 12]);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_slice_is_empty_because_that_is_the_teardown_working() {
        let pids = collect_cgroup_pids(Path::new("/nonexistent/outage")).unwrap();
        assert!(pids.is_empty());
    }

    #[test]
    fn an_unreadable_slice_is_an_error_not_an_empty_one() {
        use std::os::unix::fs::PermissionsExt;

        let root = scratch("cg-denied");
        let hidden = root.join("session.scope");
        fs::create_dir_all(&hidden).unwrap();
        fs::write(hidden.join("cgroup.procs"), "99\n").unwrap();
        fs::set_permissions(&hidden, fs::Permissions::from_mode(0o000)).unwrap();

        let result = collect_cgroup_pids(&root);
        if fs::read_dir(&hidden).is_ok() {
            fs::set_permissions(&hidden, fs::Permissions::from_mode(0o755)).unwrap();
            fs::remove_dir_all(&root).unwrap();
            return;
        }
        let err = result.expect_err("a cgroup we cannot read must never read as empty");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        fs::set_permissions(&hidden, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cgroup_membership_is_checked_against_the_slice_name() {
        let root = scratch("proc-cg");
        let pid = root.join("4242");
        fs::create_dir_all(&pid).unwrap();
        fs::write(
            pid.join("cgroup"),
            "0::/user.slice/user-1000.slice/session-3.scope\n",
        )
        .unwrap();

        assert!(in_cgroup(&root, 4242, "user-1000.slice"));
        assert!(!in_cgroup(&root, 4242, "user-1001.slice"));
        assert!(!in_cgroup(&root, 4243, "user-1000.slice"));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_owning_uid_is_read_from_the_real_uid_column() {
        let root = scratch("proc-uid");
        let pid = root.join("77");
        fs::create_dir_all(&pid).unwrap();
        fs::write(
            pid.join("status"),
            "Name:\tzsh\nState:\tS (sleeping)\nUid:\t1000\t1000\t1000\t1000\nGid:\t100\t100\t100\t100\n",
        )
        .unwrap();

        assert_eq!(proc_uid(&root, 77), Some(1000));
        assert_eq!(proc_uid(&root, 78), None);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_pid_that_left_the_slice_is_not_signalled() {
        let root = scratch("proc-moved");
        let pid = root.join("4242");
        fs::create_dir_all(&pid).unwrap();
        fs::write(pid.join("cgroup"), "0::/system.slice/sshd.service\n").unwrap();
        fs::write(pid.join("status"), "Uid:\t0\t0\t0\t0\n").unwrap();

        let outcome = kill_in_slice(&root, std::process::id() as i32, 1000, "user-1000.slice");
        assert!(matches!(outcome, Killed::Skipped));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_helper_that_hangs_is_killed_at_the_timeout() {
        let started = Instant::now();
        let err = run_bounded("/bin/sh", &["-c", "sleep 30"], Duration::from_millis(300))
            .expect_err("a hung helper must not be waited on forever");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_helper_that_fails_reports_its_exit_status() {
        let err = run_bounded("/bin/sh", &["-c", "exit 7"], Duration::from_secs(5))
            .expect_err("a non-zero exit must not be swallowed");
        assert!(err.to_string().contains("exited"), "{err}");
    }

    #[test]
    fn a_helper_that_does_not_exist_is_an_error() {
        let err = run_bounded("/nonexistent/outage-helper", &[], Duration::from_secs(1))
            .expect_err("a missing helper must not look like success");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
