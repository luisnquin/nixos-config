use std::ffi::c_int;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

/// Overrides the `ee-freecad-server` on PATH. Set by the wrapper the packaging
/// installs, which names the server by store path, so `ee` starts the one it was
/// built against and not whatever a shell happens to have inherited.
pub const SERVER_ENV: &str = "EE_WORKBENCH_CAD_SERVER";

/// The identity of that same server, set by the same wrapper from the same
/// derivation. Read rather than derived from `SERVER_ENV`: the binary sits
/// behind a symlink into a FreeCAD-shaped home, so the path `ee` executes is not
/// the path the server reports.
pub const BUILD_ENV: &str = "EE_WORKBENCH_CAD_BUILD";

/// Set to 0, no or never to keep `ee` from starting a session by itself.
pub const AUTOSTART_ENV: &str = "EE_WORKBENCH_CAD_AUTOSTART";

/// FreeCAD's own init dominates this: loading Part, Sketcher and PartDesign
/// takes seconds on a cold page cache, and a client that gives up early leaves
/// a server nobody is talking to.
const READY_TIMEOUT: Duration = Duration::from_secs(90);
const POLL_INTERVAL: Duration = Duration::from_millis(40);

/// Two `ee` processes racing a first call must not both get to bind. `flock` on
/// a side file is the whole exclusion: the kernel releases it when the holder
/// exits, so a killed `ee` cannot wedge the next one.
const LOCK_EX: c_int = 2;

unsafe extern "C" {
    fn flock(fd: c_int, operation: c_int) -> c_int;
    fn setsid() -> c_int;
}

#[derive(Debug)]
pub enum Started {
    /// Something was already listening; this call did nothing.
    Existing,
    /// This call spawned the server and waited for it to answer.
    Spawned,
}

fn responding(socket: &Path) -> bool {
    UnixStream::connect(socket).is_ok()
}

fn autostart_allowed() -> bool {
    match std::env::var(AUTOSTART_ENV) {
        Ok(value) => !matches!(value.trim(), "0" | "no" | "never" | "false"),
        Err(_) => true,
    }
}

fn server_binary() -> PathBuf {
    match std::env::var_os(SERVER_ENV).filter(|value| !value.is_empty()) {
        Some(value) => PathBuf::from(value),
        None => PathBuf::from("ee-freecad-server"),
    }
}

/// Which server this `ee` was packaged with, or nothing when it was not
/// packaged at all — a `cargo run` has no answer to give, and a check with no
/// expectation has to stay quiet rather than refuse everything.
pub fn expected_build() -> Option<String> {
    std::env::var(BUILD_ENV)
        .ok()
        .filter(|value| !value.is_empty())
}

fn log_path(socket: &Path) -> PathBuf {
    let mut path = socket.to_path_buf();
    path.set_extension("log");
    path
}

/// The socket, its lock and its log all live here, and on a fresh boot nobody
/// has made it yet.
fn prepare(socket: &Path) -> Result<()> {
    let Some(parent) = socket.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))
}

/// Held for as long as the guard lives; dropping the file releases it.
struct Lock {
    _file: File,
}

impl Lock {
    fn take(socket: &Path) -> Result<Self> {
        let mut path = socket.to_path_buf();
        path.set_extension("spawn.lock");

        let file = File::options()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening the spawn lock {}", path.display()))?;

        // Blocking on purpose: the loser waits for the winner's server rather
        // than reporting a failure the user would only retry.
        if unsafe { flock(file.as_raw_fd(), LOCK_EX) } != 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("locking {}", path.display()));
        }

        Ok(Self { _file: file })
    }
}

fn tail(path: &Path) -> String {
    const KEEP: u64 = 4096;

    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    if length > KEEP && file.seek(SeekFrom::End(-(KEEP as i64))).is_err() {
        return String::new();
    }

    let mut text = String::new();
    if file.read_to_string(&mut text).is_err() {
        return String::new();
    }

    text.lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

fn launch(socket: &Path, log: &Path) -> Result<Child> {
    // Truncated rather than appended: the log describes the session that is
    // starting now, and a diagnosis after a failed spawn should not have to
    // find the boundary between runs.
    let out = File::create(log).with_context(|| format!("creating {}", log.display()))?;
    let err = out.try_clone().context("cloning the server log handle")?;

    let binary = server_binary();
    let mut command = Command::new(&binary);
    command
        .arg("--socket")
        .arg(socket)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err);

    // Its own session, so a Ctrl-C aimed at the `ee` that happened to start it
    // does not take the documents down with it.
    unsafe {
        command.pre_exec(|| {
            if setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "{} is not on PATH: install ee-workbench with programs.ee-workbench.cad.enable, \
                 or point {SERVER_ENV} at the binary",
                binary.display()
            )
        } else {
            anyhow::Error::new(error).context(format!("starting {}", binary.display()))
        }
    })
}

/// Makes sure something is listening on `socket`, starting a session if not.
/// Every `ee mechanical` verb but `status` goes through here, so an agent never
/// has to ask anyone to start FreeCAD for it.
pub fn ensure(socket: &Path) -> Result<Started> {
    if responding(socket) {
        return Ok(Started::Existing);
    }

    if !autostart_allowed() {
        bail!(
            "no cad session on {} and {AUTOSTART_ENV} forbids starting one: \
             run `ee mechanical session start` or unset it",
            socket.display()
        );
    }

    prepare(socket)?;
    let _lock = Lock::take(socket)?;

    // The lock may have been held by another `ee` that just finished starting
    // the very server this call wanted.
    if responding(socket) {
        return Ok(Started::Existing);
    }

    let log = log_path(socket);
    let mut child = launch(socket, &log)?;

    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if responding(socket) {
            return Ok(Started::Spawned);
        }

        if let Some(status) = child.try_wait().context("waiting on ee-freecad-server")? {
            let reason = tail(&log);
            let detail = if reason.is_empty() {
                format!("see {}", log.display())
            } else {
                reason
            };
            bail!("ee-freecad-server exited ({status}) before it was ready:\n{detail}");
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            bail!(
                "ee-freecad-server did not answer on {} within {} seconds; see {}",
                socket.display(),
                READY_TIMEOUT.as_secs(),
                log.display()
            );
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Where a failed spawn left its output, for a message that can point at it.
pub fn log_for(socket: &Path) -> PathBuf {
    log_path(socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ee-spawn-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("cad.sock")
    }

    /// One test, because the knobs are process-wide environment variables and
    /// cargo runs test functions in parallel threads.
    #[test]
    fn spawning_probes_first_and_reports_a_server_that_dies() {
        let listening = scratch("existing");
        let _listener = UnixListener::bind(&listening).unwrap();

        // A binary that cannot exist: reaching the launch path at all would
        // turn this into a failure rather than an `Existing`.
        unsafe { std::env::set_var(SERVER_ENV, "/nonexistent/ee-freecad-server") };
        assert!(matches!(ensure(&listening).unwrap(), Started::Existing));

        let dead = scratch("dies");
        unsafe { std::env::set_var(SERVER_ENV, "/bin/sh") };
        let error = ensure(&dead).unwrap_err();
        assert!(error.to_string().contains("before it was ready"), "{error}");

        unsafe { std::env::set_var(AUTOSTART_ENV, "0") };
        let error = ensure(&scratch("refused")).unwrap_err();
        assert!(error.to_string().contains("forbids"), "{error}");

        unsafe { std::env::remove_var(AUTOSTART_ENV) };
        unsafe { std::env::remove_var(SERVER_ENV) };

        // A fresh boot has no runtime directory at all, and the lock is taken
        // before anything else would have made one.
        let nested = scratch("nested").parent().unwrap().join("deep/cad.sock");
        unsafe { std::env::set_var(SERVER_ENV, "/bin/sh") };
        let error = ensure(&nested).unwrap_err();
        assert!(error.to_string().contains("before it was ready"), "{error}");
        assert!(nested.parent().unwrap().is_dir());
        unsafe { std::env::remove_var(SERVER_ENV) };
    }
}
