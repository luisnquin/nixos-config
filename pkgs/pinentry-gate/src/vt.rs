use std::os::unix::io::AsFd;

use linux_raw_sys::ioctl::{VT_ACTIVATE, VT_WAITACTIVE};
use rustix::io;
use rustix::ioctl::{ioctl, IntegerSetter, Opcode};

pub fn active() -> Option<u16> {
    let name = std::fs::read_to_string("/sys/class/tty/tty0/active").ok()?;
    name.trim_end().strip_prefix("tty")?.parse().ok()
}

pub fn activate(fd: impl AsFd, vt: u16) -> io::Result<()> {
    console_ioctl::<VT_ACTIVATE>(fd, vt)
}

pub fn wait_active(fd: impl AsFd, vt: u16) -> io::Result<()> {
    console_ioctl::<VT_WAITACTIVE>(fd, vt)
}

fn console_ioctl<const OPCODE: Opcode>(fd: impl AsFd, vt: u16) -> io::Result<()> {
    // SAFETY: only VT_ACTIVATE and VT_WAITACTIVE reach here; both take the console
    // number by value, read no user memory, and reject 0 or > MAX_NR_CONSOLES.
    unsafe { ioctl(fd, IntegerSetter::<OPCODE>::new_usize(usize::from(vt))) }
}
