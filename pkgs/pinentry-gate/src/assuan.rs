//! The pinentry side of the Assuan protocol, as gpg-agent speaks it.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};

pub const CANCELLED: &str = "ERR 83886179 Operation cancelled <Pinentry>";
pub const NOT_CONFIRMED: &str = "ERR 83886194 Not confirmed <Pinentry>";
pub const UNKNOWN_COMMAND: &str = "ERR 83886355 Unknown IPC command <Pinentry>";

pub fn escape(value: &str) -> String {
    let mut out = Vec::with_capacity(value.len());
    for byte in value.bytes() {
        if byte == b'%' || byte < 0x20 {
            out.extend_from_slice(format!("%{byte:02X}").as_bytes());
        } else {
            out.push(byte);
        }
    }
    String::from_utf8(out).expect("escaped text keeps its utf-8")
}

pub fn unescape(value: &[u8]) -> String {
    let mut out = Vec::with_capacity(value.len());
    let mut i = 0;
    while i < value.len() {
        if value[i] == b'%' && i + 2 < value.len() {
            if let Ok(hex) = std::str::from_utf8(&value[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(value[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What a surface is shown: the request minus what only the agent cares for.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Payload {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub ok: String,
    #[serde(default)]
    pub cancel: String,
}

/// What one prompt has to show. Every field is set by the agent through a
/// SET* command before GETPIN or CONFIRM; the mode is which of those came.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Request {
    pub mode: String,
    pub title: String,
    pub desc: String,
    pub prompt: String,
    pub error: String,
    pub ok: String,
    pub cancel: String,
    pub notok: String,
    pub keyinfo: String,
    pub ttyname: String,
}

impl Request {
    pub fn payload(&self) -> Payload {
        Payload {
            mode: self.mode.clone(),
            title: self.title.clone(),
            desc: self.desc.clone(),
            prompt: self.prompt.clone(),
            error: self.error.clone(),
            ok: self.ok.clone(),
            cancel: self.cancel.clone(),
        }
    }
}

const IGNORED: &[&str] = &[
    "SETQUALITYBAR",
    "SETQUALITYBAR_TT",
    "SETREPEAT",
    "SETREPEATERROR",
    "SETREPEATOK",
    "SETTIMEOUT",
    "SETGENPIN",
    "SETGENPIN_TT",
    "CLEARPASSPHRASE",
    "NOP",
    "CANCEL",
    "END",
    "HELP",
];

/// None is a cancel; for confirm and message any string is a yes.
pub type Asker<'a> = Box<dyn FnMut(&Request) -> Option<String> + 'a>;

pub struct Server<'a, R: BufRead, W: Write> {
    ask: Asker<'a>,
    inp: R,
    out: W,
    request: Request,
    options: HashMap<String, String>,
}

impl<'a, R: BufRead, W: Write> Server<'a, R, W> {
    pub fn new(ask: impl FnMut(&Request) -> Option<String> + 'a, inp: R, out: W) -> Self {
        Server {
            ask: Box::new(ask),
            inp,
            out,
            request: Request::default(),
            options: HashMap::new(),
        }
    }

    fn send(&mut self, line: &str) -> io::Result<()> {
        self.out.write_all(line.as_bytes())?;
        self.out.write_all(b"\n")?;
        self.out.flush()
    }

    pub fn serve(&mut self) -> io::Result<()> {
        self.send("OK Pleased to meet you")?;
        let mut raw = Vec::new();
        loop {
            raw.clear();
            if self.inp.read_until(b'\n', &mut raw)? == 0 {
                return Ok(());
            }
            while raw.last().is_some_and(|b| *b == b'\n' || *b == b'\r') {
                raw.pop();
            }
            if raw.is_empty() || raw[0] == b'#' {
                continue;
            }
            let split = raw.iter().position(|b| *b == b' ').unwrap_or(raw.len());
            let cmd = String::from_utf8_lossy(&raw[..split]).into_owned();
            let arg = raw[split..].to_vec();
            if !self.handle(&cmd, &arg)? {
                return Ok(());
            }
        }
    }

    fn handle(&mut self, cmd: &str, arg: &[u8]) -> io::Result<bool> {
        let text = unescape(arg).trim().to_string();
        match cmd {
            "BYE" => {
                self.send("OK closing connection")?;
                return Ok(false);
            }
            "OPTION" => {
                let (key, value) = text.split_once('=').unwrap_or((&text, ""));
                let (key, value) = (key.trim().to_string(), value.trim().to_string());
                if key == "ttyname" {
                    self.request.ttyname = value.clone();
                }
                self.options.insert(key, value);
                self.send("OK")?;
            }
            "GETINFO" => self.getinfo(&text)?,
            "SETTITLE" => self.set(|r| &mut r.title, text)?,
            "SETDESC" => self.set(|r| &mut r.desc, text)?,
            "SETPROMPT" => self.set(|r| &mut r.prompt, text)?,
            "SETERROR" => self.set(|r| &mut r.error, text)?,
            "SETOK" => self.set(|r| &mut r.ok, text)?,
            "SETCANCEL" => self.set(|r| &mut r.cancel, text)?,
            "SETNOTOK" => self.set(|r| &mut r.notok, text)?,
            "SETKEYINFO" => self.set(|r| &mut r.keyinfo, text)?,
            "RESET" => {
                let ttyname = std::mem::take(&mut self.request.ttyname);
                self.request = Request { ttyname, ..Request::default() };
                self.send("OK")?;
            }
            "GETPIN" => self.getpin()?,
            "CONFIRM" => self.confirm(text.contains("--one-button"))?,
            "MESSAGE" => self.confirm(true)?,
            _ if IGNORED.contains(&cmd) => self.send("OK")?,
            _ => self.send(UNKNOWN_COMMAND)?,
        }
        Ok(true)
    }

    fn set(&mut self, field: impl FnOnce(&mut Request) -> &mut String, value: String) -> io::Result<()> {
        *field(&mut self.request) = value;
        self.send("OK")
    }

    fn getinfo(&mut self, what: &str) -> io::Result<()> {
        let option = |key: &str| self.options.get(key).cloned().unwrap_or_else(|| "-".into());
        let answer = match what {
            "pid" => Some(std::process::id().to_string()),
            "version" => Some(env!("CARGO_PKG_VERSION").to_string()),
            "flavor" => Some("pinentry-gate".to_string()),
            "ttyinfo" => Some(format!("{} {} {}", option("ttyname"), option("ttytype"), option("display"))),
            _ => None,
        };
        if let Some(answer) = answer {
            self.send(&format!("D {}", escape(&answer)))?;
        }
        self.send("OK")
    }

    fn getpin(&mut self) -> io::Result<()> {
        self.request.mode = "pin".into();
        let answer = (self.ask)(&self.request);
        // gpg-agent sets the error again before every retry; a stale one would
        // otherwise sit on the next prompt of the same connection.
        self.request.error.clear();
        match answer {
            None => self.send(CANCELLED),
            Some(pin) => {
                self.send(&format!("D {}", escape(&pin)))?;
                self.send("OK")
            }
        }
    }

    fn confirm(&mut self, one_button: bool) -> io::Result<()> {
        self.request.mode = if one_button { "message" } else { "confirm" }.into();
        let answer = (self.ask)(&self.request);
        self.request.error.clear();
        if answer.is_none() && !one_button {
            return self.send(NOT_CONFIRMED);
        }
        self.send("OK")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn transcript(script: &str, mut ask: impl FnMut(&Request) -> Option<String>) -> Vec<String> {
        let mut out = Vec::new();
        {
            let mut server = Server::new(&mut ask, Cursor::new(script.as_bytes().to_vec()), &mut out);
            server.serve().unwrap();
        }
        String::from_utf8(out).unwrap().lines().map(str::to_string).collect()
    }

    #[test]
    fn escape_round_trip() {
        let text = "a%b\r\nc\tñ";
        assert_eq!(unescape(escape(text).as_bytes()), text);
        assert_eq!(escape("50%"), "50%25");
    }

    #[test]
    fn getpin_answers_with_the_pin_and_clears_the_error() {
        let mut seen = Vec::new();
        let lines = transcript(
            "OPTION ttyname=/dev/pts/3\nSETDESC Please enter the passphrase%0A  SHA256:x\nSETPROMPT Passphrase:\nSETERROR Bad Passphrase (try 2 of 3)\nGETPIN\nGETPIN\nBYE\n",
            |request| {
                seen.push(request.clone());
                Some("hunter2".into())
            },
        );
        assert_eq!(lines[0], "OK Pleased to meet you");
        assert!(lines.contains(&"D hunter2".to_string()));
        assert_eq!(lines.last().unwrap(), "OK closing connection");
        assert_eq!(seen[0].desc, "Please enter the passphrase\n  SHA256:x");
        assert_eq!(seen[0].error, "Bad Passphrase (try 2 of 3)");
        assert_eq!(seen[0].ttyname, "/dev/pts/3");
        assert_eq!(seen[0].mode, "pin");
        assert_eq!(seen[1].error, "");
    }

    #[test]
    fn cancel_is_the_agents_cancel_error() {
        let lines = transcript("GETPIN\nBYE\n", |_| None);
        assert!(lines.contains(&CANCELLED.to_string()));
        assert!(!lines.iter().any(|line| line.starts_with("D ")));
    }

    #[test]
    fn pin_with_newline_is_escaped() {
        let lines = transcript("GETPIN\n", |_| Some("a\nb%".into()));
        assert!(lines.contains(&"D a%0Ab%25".to_string()));
    }

    #[test]
    fn confirm_modes() {
        let mut modes = Vec::new();
        let lines = transcript("CONFIRM\nCONFIRM --one-button\nMESSAGE\n", |request| {
            modes.push(request.mode.clone());
            None
        });
        assert_eq!(modes, vec!["confirm", "message", "message"]);
        assert_eq!(lines.iter().filter(|line| *line == NOT_CONFIRMED).count(), 1);
        assert!(lines.iter().filter(|line| *line == "OK").count() >= 2);
    }

    #[test]
    fn getinfo_and_unknown() {
        let lines = transcript("GETINFO flavor\nGETINFO pid\nFROBNICATE\nNOP\n", |_| Some("x".into()));
        assert!(lines.contains(&"D pinentry-gate".to_string()));
        assert!(lines.iter().any(|line| line.starts_with("ERR 83886355")));
    }

    #[test]
    fn payload_carries_what_a_surface_shows() {
        let request = Request { desc: "d".into(), error: "e".into(), keyinfo: "k".into(), ..Request::default() };
        let json = serde_json::to_string(&request.payload()).unwrap();
        assert!(json.contains("\"desc\":\"d\""));
        assert!(!json.contains("keyinfo"));
        let back: Payload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.error, "e");
    }
}
