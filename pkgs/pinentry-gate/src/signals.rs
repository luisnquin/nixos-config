//! SIGTERM and SIGHUP as a flag the loops read, so a request the agent gave up
//! on tears its surfaces down instead of leaving a console switched away.

use std::sync::atomic::{AtomicBool, Ordering};

pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn mark(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

pub fn install() {
    let handler = mark as extern "C" fn(libc::c_int) as libc::sighandler_t;
    unsafe {
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGHUP, handler);
    }
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}
