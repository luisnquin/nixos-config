//! SIGTERM, SIGHUP and SIGINT as a flag the loops read, so a request the agent
//! gave up on tears its surfaces down instead of leaving a console switched
//! away. SIGINT is how gpg-agent cancels once its client hangs up.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

pub static INTERRUPTED: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

pub fn install() {
    for signal in [SIGTERM, SIGHUP, SIGINT] {
        let _ = signal_hook::flag::register(signal, Arc::clone(&INTERRUPTED));
    }
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}
