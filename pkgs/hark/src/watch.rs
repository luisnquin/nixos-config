use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

pub const DBUS_MONITOR: &str = "@dbus_monitor@";

const MAKO_PATH: &str = "/fr/emersion/Mako";

/// mako publishes `Notifications` and `Modes` as emits-invalidation D-Bus
/// properties, so the signal carries no payload — it is purely a "look again"
/// tap, which is all a redraw needs.
pub fn run(mut emit: impl FnMut()) -> ! {
    let (tx, rx) = mpsc::channel();
    spawn_bus_watch(tx.clone());
    spawn_marker_watch(tx);

    emit();
    loop {
        // The periodic wake is what keeps "5m ago" honest while nothing at all
        // is happening on the bus.
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(()) => {
                // A single user action fans out into several signals; redrawing
                // once at the end of the burst is enough.
                while rx.recv_timeout(Duration::from_millis(120)).is_ok() {}
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => thread::sleep(Duration::from_secs(30)),
        }
        emit();
    }
}

fn spawn_bus_watch(tx: Sender<()>) {
    thread::spawn(move || loop {
        let child = Command::new(DBUS_MONITOR)
            .arg("--session")
            .arg(format!("type='signal',path='{MAKO_PATH}'"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        if let Ok(mut child) = child {
            if let Some(stdout) = child.stdout.take() {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if line.contains("PropertiesChanged") && tx.send(()).is_err() {
                        return;
                    }
                }
            }
            let _ = child.wait();
        }
        // The bus itself went away, or was never there; try again rather than
        // degrading the module to a dead icon.
        thread::sleep(Duration::from_secs(2));
    });
}

fn spawn_marker_watch(tx: Sender<()>) {
    thread::spawn(move || {
        let Some(path) = crate::state::marker() else {
            return;
        };
        // Seeded with what is already on disk: a redraw at startup is the
        // caller's job, not a phantom change event.
        let mut last = std::fs::metadata(&path).and_then(|meta| meta.modified()).ok();
        loop {
            let stamp = std::fs::metadata(&path).and_then(|meta| meta.modified()).ok();
            if stamp != last {
                last = stamp;
                if tx.send(()).is_err() {
                    return;
                }
            }
            thread::sleep(Duration::from_millis(400));
        }
    });
}
