//! One request, every surface, first decision wins.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::assuan::Request;
use crate::config::Config;
use crate::signals::interrupted;
use crate::{phone, seat};

/// A surface is started with the fifo it answers on and the file describing
/// the request; it hands back the process, or None when it cannot start.
pub type Surface = Box<dyn FnOnce(&Path, &Path) -> Option<Child>>;

pub fn runtime_dir() -> PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    PathBuf::from(base).join("pinentry-gate")
}

fn modal_args(fifo: &Path, request_file: &Path) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = vec!["modal".into(), "--answer".into(), fifo.into(), "--request".into(), request_file.into()];
    args.shrink_to_fit();
    args
}

pub fn window_surface(config: &Config, runtime: &Path) -> Option<Surface> {
    let terminal = config.terminal.clone();
    let (program, rest) = terminal.split_first()?;
    let display = seat::wayland_display(runtime)?;
    let (program, rest) = (program.clone(), rest.to_vec());
    Some(Box::new(move |fifo, request_file| {
        let exe = std::env::current_exe().ok()?;
        Command::new(program)
            .args(rest)
            .arg(exe)
            .args(modal_args(fifo, request_file))
            .env("WAYLAND_DISPLAY", display)
            .stdin(Stdio::null())
            .spawn()
            .ok()
    }))
}

pub fn vt_surface(config: &Config) -> Option<Surface> {
    let vt = config.vt?;
    let device = std::ffi::CString::new(format!("/dev/tty{vt}")).ok()?;
    if unsafe { libc::access(device.as_ptr(), libc::R_OK | libc::W_OK) } != 0 {
        return None;
    }
    Some(Box::new(move |fifo, request_file| {
        let exe = std::env::current_exe().ok()?;
        let mut command = Command::new(exe);
        command.args(modal_args(fifo, request_file)).arg("--vt").arg(vt.to_string()).stdin(Stdio::null());
        // Its own session, so the console can become its controlling tty and
        // the switch ioctls are permitted without any capability.
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        command.spawn().ok()
    }))
}

/// The computer's surfaces in the order they are tried. The window comes
/// first only while the seat is on this user's graphical session; a window on
/// a compositor sitting behind a text console would be shown to nobody.
pub fn local_surfaces(config: &Config, runtime: &Path) -> Vec<Surface> {
    let mut chain = Vec::new();
    if seat::active_session_kind(&config.user) == seat::Kind::Graphical {
        chain.extend(window_surface(config, runtime.parent().unwrap_or(runtime)));
    }
    chain.extend(vt_surface(config));
    chain
}

fn next(surfaces: &mut Vec<Surface>, fifo: &Path, request_file: &Path) -> Option<Child> {
    while !surfaces.is_empty() {
        if let Some(child) = surfaces.remove(0)(fifo, request_file) {
            return Some(child);
        }
    }
    None
}

fn stop(child: &mut Option<Child>) {
    let Some(mut proc) = child.take() else { return };
    if proc.try_wait().ok().flatten().is_some() {
        return;
    }
    unsafe { libc::kill(proc.id() as i32, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if proc.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = proc.kill();
    let _ = proc.wait();
}

fn random_id() -> String {
    let mut bytes = [0u8; 8];
    if File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes)).is_err() {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        bytes = now.as_nanos().to_le_bytes()[..8].try_into().unwrap();
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

type Surfaces = Box<dyn Fn(&Config, &Path) -> Vec<Surface>>;
type Notify = Box<dyn Fn(&Path, &str, &str) -> Vec<PathBuf>>;
type Done = Box<dyn Fn(&[PathBuf], &str)>;

pub struct Broker {
    config: Config,
    runtime: PathBuf,
    surfaces: Surfaces,
    notify: Notify,
    done: Done,
}

impl Broker {
    pub fn new(config: Config) -> Broker {
        Broker::with(
            config,
            runtime_dir(),
            Box::new(local_surfaces),
            Box::new(phone::notify),
            Box::new(phone::done),
        )
    }

    pub fn with(config: Config, runtime: PathBuf, surfaces: Surfaces, notify: Notify, done: Done) -> Broker {
        Broker { config, runtime, surfaces, notify, done }
    }

    pub fn ask(&mut self, request: &Request) -> Option<String> {
        if fs::create_dir_all(&self.runtime).is_err() {
            return None;
        }
        let _ = fs::set_permissions(&self.runtime, fs::Permissions::from_mode(0o700));
        let id = random_id();
        let fifo = self.runtime.join(&id);
        let request_file = self.runtime.join(format!("{id}.json"));
        let Ok(path) = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()) else {
            return None;
        };
        if unsafe { libc::mkfifo(path.as_ptr(), 0o600) } != 0 {
            return None;
        }
        let reader = OpenOptions::new().read(true).write(true).custom_flags(libc::O_NONBLOCK).open(&fifo);
        let Ok(mut reader) = reader else {
            let _ = fs::remove_file(&fifo);
            return None;
        };
        let payload = serde_json::to_string(&request.payload()).unwrap_or_default();
        let written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&request_file)
            .and_then(|mut file| file.write_all(payload.as_bytes()));
        let answer = if written.is_ok() {
            let mut surfaces = (self.surfaces)(&self.config, &self.runtime);
            let ptys = (self.notify)(&self.runtime, &id, &payload);
            let mut child = next(&mut surfaces, &fifo, &request_file);
            let answer = if child.is_none() && ptys.is_empty() {
                None
            } else {
                self.wait(&mut reader, &mut child, &mut surfaces, &fifo, &request_file, ptys.is_empty())
            };
            stop(&mut child);
            (self.done)(&ptys, &id);
            answer
        } else {
            None
        };
        drop(reader);
        let _ = fs::remove_file(&fifo);
        let _ = fs::remove_file(&request_file);
        answer
    }

    fn wait(
        &self,
        reader: &mut File,
        child: &mut Option<Child>,
        surfaces: &mut Vec<Surface>,
        fifo: &Path,
        request_file: &Path,
        no_phones: bool,
    ) -> Option<String> {
        let mut buf = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(self.config.timeout);
        while Instant::now() < deadline && !interrupted() {
            let mut pfd = libc::pollfd { fd: reader.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            let ready = unsafe { libc::poll(&mut pfd, 1, 100) };
            if ready > 0 {
                let mut chunk = [0u8; 4096];
                if let Ok(n) = reader.read(&mut chunk) {
                    buf.extend_from_slice(&chunk[..n]);
                }
                if let Some(end) = buf.iter().position(|b| *b == b'\n') {
                    let line = String::from_utf8_lossy(&buf[..end]).into_owned();
                    return if line.is_empty() { None } else { Some(line) };
                }
            }
            let died = child.as_mut().is_some_and(|proc| proc.try_wait().ok().flatten().is_some());
            if died {
                // Died without deciding: the compositor refused the window,
                // the console could not be taken. The next surface gets its
                // turn.
                *child = next(surfaces, fifo, request_file);
                if child.is_none() && no_phones {
                    return None;
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn shell(script: &'static str) -> Surface {
        Box::new(move |fifo, request_file| {
            Command::new("sh")
                .arg("-c")
                .arg(script)
                .env("FIFO", fifo)
                .env("REQ", request_file)
                .spawn()
                .ok()
        })
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pinentry-gate-broker-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn broker(name: &str, surfaces: Vec<&'static str>, timeout: u64) -> Broker {
        let scripts = Arc::new(Mutex::new(surfaces));
        Broker::with(
            Config { user: "me".into(), timeout, ..Config::default() },
            scratch(name),
            Box::new(move |_, _| scripts.lock().unwrap().iter().map(|s| shell(s)).collect()),
            Box::new(|_, _, _| Vec::new()),
            Box::new(|_, _| {}),
        )
    }

    fn leftovers(broker: &Broker) -> usize {
        std::fs::read_dir(&broker.runtime).map(|d| d.count()).unwrap_or(0)
    }

    #[test]
    fn local_surface_answer_wins() {
        let mut b = broker("wins", vec!["printf 'secret\\n' > \"$FIFO\""], 5);
        assert_eq!(b.ask(&Request { desc: "d".into(), ..Request::default() }), Some("secret".into()));
        assert_eq!(leftovers(&b), 0);
    }

    #[test]
    fn request_file_carries_the_payload() {
        let mut b = broker("payload", vec!["cat \"$REQ\" > \"$FIFO\"; printf '\\n' > \"$FIFO\""], 5);
        let answer = b.ask(&Request { desc: "hello".into(), error: "bad".into(), ..Request::default() }).unwrap();
        assert!(answer.contains("\"desc\":\"hello\""));
        assert!(answer.contains("\"error\":\"bad\""));
    }

    #[test]
    fn a_surface_that_dies_hands_over_to_the_next() {
        let mut b = broker("handover", vec!["exit 3", "printf 'second\\n' > \"$FIFO\""], 5);
        assert_eq!(b.ask(&Request::default()), Some("second".into()));
    }

    #[test]
    fn empty_line_is_a_cancel() {
        let mut b = broker("cancel", vec!["printf '\\n' > \"$FIFO\""], 5);
        assert_eq!(b.ask(&Request::default()), None);
    }

    #[test]
    fn no_surface_at_all_cancels_at_once() {
        let mut b = broker("none", vec![], 5);
        let started = Instant::now();
        assert_eq!(b.ask(&Request::default()), None);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn timeout_cancels_and_stops_the_surface() {
        let mut b = broker("timeout", vec!["sleep 30"], 1);
        let started = Instant::now();
        assert_eq!(b.ask(&Request::default()), None);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(leftovers(&b), 0);
    }

    #[test]
    fn phone_answer_wins_and_the_modal_is_told() {
        let told = Arc::new(Mutex::new(Vec::new()));
        let seen = told.clone();
        let runtime = scratch("phone");
        let answered = runtime.clone();
        let mut b = Broker::with(
            Config { user: "me".into(), timeout: 5, ..Config::default() },
            runtime,
            Box::new(|_, _| vec![shell("sleep 30")]),
            Box::new(move |_, _, _| {
                let dir = answered.clone();
                std::thread::spawn(move || {
                    for _ in 0..100 {
                        let fifo = std::fs::read_dir(&dir).ok().and_then(|d| {
                            d.flatten().map(|e| e.path()).find(|p| p.extension().is_none())
                        });
                        if let Some(fifo) = fifo {
                            std::fs::write(fifo, "from-phone\n").unwrap();
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                });
                vec![PathBuf::from("/dev/pts/99")]
            }),
            Box::new(move |ptys, id| seen.lock().unwrap().push((ptys.to_vec(), id.to_string()))),
        );
        let started = Instant::now();
        assert_eq!(b.ask(&Request::default()), Some("from-phone".into()));
        assert!(started.elapsed() < Duration::from_secs(5));
        let told = told.lock().unwrap();
        assert_eq!(told[0].0, vec![PathBuf::from("/dev/pts/99")]);
    }

    #[test]
    fn phone_only_request_waits_for_the_phone() {
        let mut b = Broker::with(
            Config { user: "me".into(), timeout: 1, ..Config::default() },
            scratch("phone-only"),
            Box::new(|_, _| Vec::new()),
            Box::new(|_, _, _| vec![PathBuf::from("/dev/pts/99")]),
            Box::new(|_, _| {}),
        );
        let started = Instant::now();
        assert_eq!(b.ask(&Request::default()), None);
        assert!(started.elapsed() >= Duration::from_millis(900));
    }
}
