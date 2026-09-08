//! The modal on the computer: a box on a terminal that is not the one asking,
//! either the terminal window the broker opened or a reserved virtual
//! console it switches to.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::time::Duration;

use crate::assuan::Payload;
use crate::signals::{self, interrupted};

const VT_GETSTATE: libc::c_ulong = 0x5603;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
const VT_WAITACTIVE: libc::c_ulong = 0x5607;

#[repr(C)]
struct VtStat {
    v_active: u16,
    v_signal: u16,
    v_state: u16,
}

pub enum Outcome {
    Decided(Option<String>),
    Interrupted,
}

/// The terminal the modal draws on, with what it has to put back.
pub struct Console {
    fd: OwnedFd,
    saved: libc::termios,
    previous_vt: Option<u16>,
}

impl Console {
    pub fn take(device: &Path, vt: Option<u16>) -> Option<Console> {
        let file = OpenOptions::new().read(true).write(true).open(device).ok()?;
        let fd = OwnedFd::from(file);
        let raw = fd.as_raw_fd();
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(raw, &mut saved) } != 0 {
            return None;
        }
        let mut previous_vt = None;
        if let Some(vt) = vt {
            // Steal it even if a controlling tty already exists; the broker
            // started this process in its own session for exactly this.
            if unsafe { libc::ioctl(raw, libc::TIOCSCTTY as _, 1) } != 0 {
                return None;
            }
            let mut state = VtStat { v_active: 0, v_signal: 0, v_state: 0 };
            if unsafe { libc::ioctl(raw, VT_GETSTATE as _, &mut state) } != 0 {
                return None;
            }
            if unsafe { libc::ioctl(raw, VT_ACTIVATE as _, vt as libc::c_ulong) } != 0 {
                return None;
            }
            unsafe { libc::ioctl(raw, VT_WAITACTIVE as _, vt as libc::c_ulong) };
            previous_vt = Some(state.v_active);
        }
        let mut mode = saved;
        unsafe { libc::cfmakeraw(&mut mode) };
        // Flushing drops keys typed before the box was up; none of them were
        // meant for it.
        unsafe { libc::tcsetattr(raw, libc::TCSAFLUSH, &mode) };
        Some(Console { fd, saved, previous_vt })
    }

    pub fn raw(&self) -> i32 {
        self.fd.as_raw_fd()
    }

    pub fn write(&self, bytes: &[u8]) {
        let mut offset = 0;
        while offset < bytes.len() {
            let n = unsafe { libc::write(self.raw(), bytes[offset..].as_ptr() as *const libc::c_void, bytes.len() - offset) };
            if n <= 0 {
                return;
            }
            offset += n as usize;
        }
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.write(b"\x1b[2J\x1b[H\x1b[?25h\x1b[?1049l");
        unsafe { libc::tcsetattr(self.raw(), libc::TCSANOW, &self.saved) };
        if let Some(previous) = self.previous_vt {
            unsafe { libc::ioctl(self.raw(), VT_ACTIVATE as _, previous as libc::c_ulong) };
        }
    }
}

pub fn size(fd: i32) -> (u16, u16) {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) } == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        (ws.ws_col, ws.ws_row)
    } else {
        (80, 24)
    }
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in text.split('\n') {
        let indent: String = raw.chars().take_while(|c| *c == ' ').collect();
        let body = raw.trim_start();
        if body.is_empty() {
            lines.push(String::new());
            continue;
        }
        let room = width.saturating_sub(indent.chars().count()).max(8);
        let mut line = String::new();
        for word in body.split(' ') {
            let candidate_len = line.chars().count() + word.chars().count() + usize::from(!line.is_empty());
            if !line.is_empty() && candidate_len > room {
                lines.push(format!("{indent}{line}"));
                line.clear();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(format!("{indent}{line}"));
    }
    lines
}

fn label(text: &str, fallback: &str) -> String {
    let cleaned: String = text.replace('_', "").trim().trim_end_matches(':').to_uppercase();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

/// A rendered request: fixed lines above the field, the field, the footer.
pub struct Frame {
    pub inner: usize,
    pub head: Vec<String>,
    pub field_label: String,
    pub footer: String,
    pub mode: String,
}

pub fn layout(payload: &Payload, cols: u16) -> Frame {
    let inner = (usize::from(cols).saturating_sub(4)).clamp(24, 72);
    let mut head = Vec::new();
    head.push(format!("\x1b[1;33m{}\x1b[0m", "AUTHORIZATION REQUIRED"));
    head.push(String::new());
    if !payload.desc.is_empty() {
        head.extend(wrap(&payload.desc, inner - 2));
        head.push(String::new());
    }
    if !payload.error.is_empty() {
        for line in wrap(&payload.error, inner - 2) {
            head.push(format!("\x1b[1;31m{line}\x1b[0m"));
        }
        head.push(String::new());
    }
    let ok = label(&payload.ok, "OK");
    let cancel = label(&payload.cancel, "CANCEL");
    let (field_label, footer) = match payload.mode.as_str() {
        "confirm" => (String::new(), format!("ENTER  <{ok}>    ESC  <{cancel}>")),
        "message" => (String::new(), format!("ENTER  {ok}")),
        _ => (label(&payload.prompt, "PASSPHRASE"), "ENTER  UNLOCK    ESC  CANCEL".to_string()),
    };
    Frame { inner, head, field_label, footer, mode: payload.mode.clone() }
}

fn visible_len(text: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in text.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            len += 1;
        }
    }
    len
}

impl Frame {
    fn field(&self, typed: usize) -> String {
        if self.field_label.is_empty() {
            return String::new();
        }
        let mask: String = "*".repeat(typed);
        let line = format!("{} > {mask}", self.field_label);
        let room = self.inner - 2;
        if visible_len(&line) > room {
            let keep = room.saturating_sub(self.field_label.len() + 4);
            format!("{} > …{}", self.field_label, "*".repeat(keep))
        } else {
            line
        }
    }

    pub fn render(&self, typed: usize, cols: u16, rows: u16) -> Vec<u8> {
        let mut body: Vec<String> = self.head.clone();
        let field = self.field(typed);
        if !field.is_empty() {
            body.push(field);
            body.push(String::new());
        }
        body.push(format!("\x1b[2m{}\x1b[0m", self.footer));
        let height = body.len() + 2;
        let left = usize::from(cols).saturating_sub(self.inner + 2) / 2;
        let top = usize::from(rows).saturating_sub(height) / 2;
        let pad = " ".repeat(left);
        let mut out = Vec::new();
        out.extend_from_slice(b"\x1b[?25l\x1b[2J\x1b[H");
        let mut lines = Vec::with_capacity(height);
        lines.push(format!("{pad}┌{}┐", "─".repeat(self.inner)));
        for line in &body {
            let fill = self.inner.saturating_sub(visible_len(line) + 2);
            lines.push(format!("{pad}│ {line}{} │", " ".repeat(fill)));
        }
        lines.push(format!("{pad}└{}┘", "─".repeat(self.inner)));
        for (i, line) in lines.iter().enumerate() {
            out.extend_from_slice(format!("\x1b[{};1H{line}", top + i + 1).as_bytes());
        }
        out
    }
}

fn read_byte(fd: i32, timeout_ms: i32) -> Option<Option<u8>> {
    let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    let ready = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if ready <= 0 {
        return if ready == 0 { Some(None) } else { None };
    }
    let mut byte = 0u8;
    let n = unsafe { libc::read(fd, &mut byte as *mut u8 as *mut libc::c_void, 1) };
    if n == 1 {
        Some(Some(byte))
    } else {
        None
    }
}

fn skip_escape_sequence(fd: i32, first: u8) {
    match first {
        b'[' => {
            while let Some(Some(byte)) = read_byte(fd, 50) {
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
        }
        b'O' => {
            read_byte(fd, 50);
        }
        _ => {}
    }
}

/// Keys until a decision. Every keystroke redraws through `draw`, which gets
/// the number of characters typed so far.
pub fn read_answer(fd: i32, mode: &str, mut draw: impl FnMut(usize)) -> Outcome {
    let mut pin: Vec<char> = Vec::new();
    let mut pending: Vec<u8> = Vec::new();
    draw(0);
    loop {
        if interrupted() {
            return Outcome::Interrupted;
        }
        let byte = match read_byte(fd, 100) {
            None => return Outcome::Interrupted,
            Some(None) => continue,
            Some(Some(byte)) => byte,
        };
        match byte {
            b'\r' | b'\n' => {
                let answer = if mode == "pin" { pin.iter().collect() } else { "ok".to_string() };
                return Outcome::Decided(Some(answer));
            }
            0x03 => return Outcome::Decided(None),
            0x1b => match read_byte(fd, 50) {
                Some(Some(next)) => skip_escape_sequence(fd, next),
                Some(None) => return Outcome::Decided(if mode == "message" { Some("ok".into()) } else { None }),
                None => return Outcome::Interrupted,
            },
            0x7f | 0x08 => {
                pin.pop();
                pending.clear();
            }
            0x15 => {
                pin.clear();
                pending.clear();
            }
            b if b < 0x20 => {}
            b => {
                pending.push(b);
                match std::str::from_utf8(&pending) {
                    Ok(text) => {
                        pin.extend(text.chars());
                        pending.clear();
                    }
                    Err(err) if err.error_len().is_some() => pending.clear(),
                    Err(_) => {}
                }
            }
        }
        draw(pin.len());
    }
}

/// Hands the decision to the broker. The fifo is opened as it is, never
/// created: once the broker has removed it another surface has already won,
/// and a passphrase must not end up in a regular file in its place.
pub fn deliver(fifo: &Path, answer: Option<String>) -> bool {
    let Ok(mut file) = OpenOptions::new().write(true).open(fifo) else {
        return false;
    };
    let mut line = answer.unwrap_or_default();
    line.push('\n');
    file.write_all(line.as_bytes()).is_ok()
}

pub fn run(console: &Console, payload: &Payload) -> Outcome {
    let (cols, rows) = size(console.raw());
    let frame = layout(payload, cols);
    console.write(b"\x1b[?1049h");
    read_answer(console.raw(), &frame.mode, |typed| console.write(&frame.render(typed, cols, rows)))
}

struct Args {
    fifo: PathBuf,
    request: PathBuf,
    vt: Option<u16>,
}

fn parse(args: &[String]) -> Option<Args> {
    let mut fifo = None;
    let mut request = None;
    let mut vt = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--answer" => fifo = Some(PathBuf::from(it.next()?)),
            "--request" => request = Some(PathBuf::from(it.next()?)),
            "--vt" => vt = Some(it.next()?.parse().ok()?),
            _ => return None,
        }
    }
    Some(Args { fifo: fifo?, request: request?, vt })
}

pub fn main(args: &[String]) -> i32 {
    signals::install();
    let Some(args) = parse(args) else {
        eprintln!("usage: pinentry-gate modal --answer FIFO --request FILE [--vt N]");
        return 2;
    };
    let payload: Payload = std::fs::read_to_string(&args.request)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let device = match args.vt {
        Some(vt) => PathBuf::from(format!("/dev/tty{vt}")),
        None => PathBuf::from("/dev/tty"),
    };
    let Some(console) = Console::take(&device, args.vt) else {
        return 3;
    };
    let outcome = run(&console, &payload);
    drop(console);
    match outcome {
        Outcome::Interrupted => 4,
        Outcome::Decided(answer) => {
            deliver(&args.fifo, answer);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pty() -> (i32, i32) {
        let mut master = 0;
        let mut slave = 0;
        let rc = unsafe { libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null(), std::ptr::null()) };
        assert_eq!(rc, 0);
        let mut mode: libc::termios = unsafe { std::mem::zeroed() };
        unsafe {
            libc::tcgetattr(slave, &mut mode);
            libc::cfmakeraw(&mut mode);
            libc::tcsetattr(slave, libc::TCSANOW, &mode);
        }
        (master, slave)
    }

    fn typed(bytes: &'static [u8]) -> Outcome {
        let (master, slave) = pty();
        let writer = std::thread::spawn(move || {
            for chunk in bytes.chunks(3) {
                unsafe { libc::write(master, chunk.as_ptr() as *const libc::c_void, chunk.len()) };
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let outcome = read_answer(slave, "pin", |_| {});
        writer.join().unwrap();
        unsafe {
            libc::close(master);
            libc::close(slave);
        }
        outcome
    }

    fn decided(outcome: Outcome) -> Option<String> {
        match outcome {
            Outcome::Decided(answer) => answer,
            Outcome::Interrupted => panic!("interrupted"),
        }
    }

    #[test]
    fn enter_submits_what_was_typed() {
        assert_eq!(decided(typed(b"hunter2\r")), Some("hunter2".into()));
    }

    #[test]
    fn utf8_and_backspace_and_clear() {
        assert_eq!(decided(typed("ñandú\x7f\r".as_bytes())), Some("ñand".into()));
        assert_eq!(decided(typed(b"abc\x15xy\r")), Some("xy".into()));
    }

    #[test]
    fn lone_escape_cancels_but_sequences_are_ignored() {
        assert_eq!(decided(typed(b"abc\x1b")), None);
        assert_eq!(decided(typed(b"a\x1b[Ab\x1bOPc\r")), Some("abc".into()));
        assert_eq!(decided(typed(b"abc\x03")), None);
    }

    #[test]
    fn confirm_answers_yes_on_enter() {
        let (master, slave) = pty();
        unsafe { libc::write(master, b"\r".as_ptr() as *const libc::c_void, 1) };
        assert_eq!(decided(read_answer(slave, "confirm", |_| {})), Some("ok".into()));
        unsafe {
            libc::close(master);
            libc::close(slave);
        }
    }

    #[test]
    fn layout_fits_the_terminal() {
        let payload = Payload {
            mode: "pin".into(),
            desc: "Please enter the passphrase for the ssh key\n  SHA256:abcdefghijklmnopqrstuvwxyz0123456789abcdefg".into(),
            prompt: "Passphrase:".into(),
            error: "Bad Passphrase (try 2 of 3)".into(),
            ..Payload::default()
        };
        let frame = layout(&payload, 60);
        assert_eq!(frame.inner, 56);
        assert_eq!(frame.field_label, "PASSPHRASE");
        assert!(frame.head.iter().any(|l| l.contains("Bad Passphrase") && l.contains("\x1b[1;31m")));
        let out = String::from_utf8(frame.render(4, 60, 24)).unwrap();
        assert!(out.contains("PASSPHRASE > ****"));
        assert!(out.contains("┌"));
        let mut lines: Vec<String> = Vec::new();
        for chunk in out.split("\x1b[") {
            let is_move = chunk.find(";1H").is_some_and(|at| chunk[..at].chars().all(|c| c.is_ascii_digit()));
            if is_move {
                lines.push(chunk[chunk.find(";1H").unwrap() + 3..].to_string());
            } else if let Some(last) = lines.last_mut() {
                last.push_str("\x1b[");
                last.push_str(chunk);
            }
        }
        assert!(lines.len() > 5);
        for line in &lines {
            assert_eq!(visible_len(line), 1 + 56 + 2, "{line:?}");
        }
        let tiny = layout(&payload, 20);
        assert_eq!(tiny.inner, 24);
    }

    #[test]
    fn confirm_footer_uses_the_agents_labels() {
        let payload = Payload { mode: "confirm".into(), ok: "_Yes".into(), cancel: "_No".into(), ..Payload::default() };
        let frame = layout(&payload, 80);
        assert_eq!(frame.footer, "ENTER  <YES>    ESC  <NO>");
        assert!(frame.field_label.is_empty());
        let message = layout(&Payload { mode: "message".into(), ..Payload::default() }, 80);
        assert_eq!(message.footer, "ENTER  OK");
    }

    #[test]
    fn deliver_never_creates_a_file() {
        let dir = std::env::temp_dir().join(format!("pinentry-gate-modal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let gone = dir.join("gone");
        assert!(!deliver(&gone, Some("secret".into())));
        assert!(!gone.exists());
    }
}
