use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{self, Unix};
use crate::registry;
use crate::ssh::Where;

pub const IDLE: Duration = Duration::from_secs(7 * 24 * 3600);
static WORKER: AtomicBool = AtomicBool::new(false);
static USED: OnceLock<Mutex<Vec<Where>>> = OnceLock::new();

#[derive(Serialize, Deserialize)]
struct Schedule {
    started: Unix,
    next: Unix,
}

impl Schedule {
    fn due(&self, now: Unix) -> bool {
        now < self.started || now >= self.next
    }
}

pub fn used(at: &Where) {
    if WORKER.load(Ordering::Relaxed) {
        return;
    }
    let mut hosts = USED
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !hosts.contains(at) {
        hosts.push(at.clone());
    }
}

pub async fn ensure_used() {
    let hosts = {
        let mut hosts = USED
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *hosts)
    };
    for at in hosts {
        if let Err(error) = ensure_host(&at).await {
            eprintln!(
                "phone: could not start cleanup on {}: {error:#}",
                at.label()
            );
        }
    }
}

async fn ensure_host(at: &Where) -> Result<()> {
    if *at == Where::Here {
        return start_due();
    }
    let script = r#"exec "${SHELL:-/bin/sh}" -l -c 'command -v phone >/dev/null 2>&1 || exit 0; exec phone daemon --ensure'"#;
    crate::ssh::landed(
        at,
        "starting cleanup",
        at.exec(script, &[], Duration::from_secs(5)).await?,
    )
}

pub(super) fn scheduled() -> bool {
    fs::read(registry::state_dir().join("cleanup.next"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Schedule>(&bytes).ok())
        .is_none_or(|schedule| schedule.due(model::now()))
}

pub fn start_due() -> Result<()> {
    if !scheduled() || !super::has_pending(IDLE)? {
        return Ok(());
    }
    fs::create_dir_all(registry::state_dir())?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(registry::state_dir().join("cleanup.log"))?;
    if log.metadata()?.len() > 256 * 1024 {
        log.set_len(0)?;
    }
    let mut child = tokio::process::Command::new(std::env::current_exe()?);
    child
        .arg("daemon")
        .arg("--once")
        .arg("--scheduled")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    #[allow(unsafe_code, reason = "std has no safe API to create a child session")]
    // SAFETY: The child callback calls only pointer-free setsid and reads errno.
    // It neither allocates nor locks between fork and exec.
    unsafe {
        child.as_std_mut().pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    child.spawn().context("spawning the cleanup worker")?;
    Ok(())
}

pub fn singleton() -> Result<File> {
    WORKER.store(true, Ordering::Relaxed);
    fs::create_dir_all(registry::state_dir())?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(registry::state_dir().join("daemon.lock"))?;
    lock.try_lock()
        .context("another phone cleanup worker is already running")?;
    Ok(lock)
}

pub fn cooldown(interval: Duration) -> Result<()> {
    let started = model::now();
    let schedule = Schedule {
        started,
        next: started.saturating_add(interval.as_secs() as i64),
    };
    let temporary = registry::state_dir().join(format!("cleanup.next.{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec(&schedule)?)?;
    fs::rename(temporary, registry::state_dir().join("cleanup.next"))?;
    Ok(())
}

pub fn deadline(timeout: Duration) -> Result<()> {
    if timeout.is_zero() {
        bail!("--timeout must be greater than zero");
    }
    #[allow(
        unsafe_code,
        reason = "std has no safe API to isolate the current process group"
    )]
    // SAFETY: These pointer-free calls use the current process and its valid PID.
    // Isolation succeeds before that PID is passed to the watchdog.
    let group = unsafe {
        let pid = libc::getpid();
        if libc::getpgrp() != pid && libc::setpgid(0, 0) == -1 {
            return Err(std::io::Error::last_os_error()).context("isolating cleanup subprocesses");
        }
        pid
    };
    #[allow(
        unsafe_code,
        reason = "best-effort worker priority requires setpriority"
    )]
    // SAFETY: PRIO_PROCESS with ID zero targets this worker after exec.
    // The priority is valid, and failure merely leaves the priority unchanged.
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
    }
    std::thread::Builder::new()
        .name("phone-cleanup-deadline".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            let reason = watch(timeout, super::memory::resident_bytes);
            std::eprintln!("phone: cleanup stopped: {reason}");
            #[allow(
                unsafe_code,
                reason = "std has no safe API to terminate a process group"
            )]
            // SAFETY: The live worker leads the isolated group with this positive PID.
            // Its negation targets only that group; kill takes no pointers.
            unsafe {
                libc::kill(-group, libc::SIGKILL);
            }
        })?;
    Ok(())
}

fn watch(timeout: Duration, resident_bytes: fn() -> Result<u64>) -> String {
    let started = Instant::now();
    loop {
        if started.elapsed() >= timeout {
            return format!("exceeded runtime limit of {}ms", timeout.as_millis());
        }
        match resident_bytes() {
            Ok(bytes) if bytes > super::memory::BUDGET => {
                return format!("resident memory {bytes} exceeds 32 MiB budget")
            }
            Ok(_) => {}
            Err(error) => return format!("cannot measure resident memory: {error:#}"),
        }
        std::thread::sleep(
            Duration::from_millis(100).min(timeout.saturating_sub(started.elapsed())),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_stops_over_budget_memory_and_zero_deadlines() {
        assert!(watch(Duration::from_secs(1), || Ok(
            super::super::memory::BUDGET + 1
        ))
        .contains("32 MiB"));
        assert!(watch(Duration::ZERO, || Ok(0)).contains("runtime limit"));
    }

    #[test]
    fn cooldown_expires_and_recovers_from_a_backward_clock_change() {
        let schedule = Schedule {
            started: 100,
            next: 200,
        };
        assert!(!schedule.due(100));
        assert!(!schedule.due(199));
        assert!(schedule.due(200));
        assert!(schedule.due(99));
    }
}
