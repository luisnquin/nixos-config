use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use tokio::io::AsyncReadExt;
use tokio::time::timeout;

use crate::actions::{capture, where_of};
use crate::adb::Server;
use crate::connect::{serial_of, Reporter};
use crate::model::{Device, Platform};
use crate::simctl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

const ENCODED_WIDTH: u32 = 432;

const PAUSE: Duration = Duration::from_millis(40);

const STALL: Duration = Duration::from_secs(3);

// hardware encoders hold the last change back until the next one, so a
// screencap stands in for it once the stream goes quiet
const SETTLE: Duration = Duration::from_millis(300);

pub enum Output {
    Raw,
    Base64,
    Shm(Frames),
}

pub struct Frames {
    name: String,
    shown: u64,
    kept: VecDeque<(u64, Instant)>,
}

const KEEP: usize = 8;

const LINGER: Duration = Duration::from_secs(1);

const SHM: &str = "/dev/shm";

fn owner(file: &str) -> Option<u32> {
    file.strip_prefix("phone.")?.split_once('.')?.0.parse().ok()
}

fn sweep_orphans(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let file = entry.file_name();
        let Some(pid) = file.to_str().and_then(owner) else { continue };
        if !Path::new("/proc").join(pid.to_string()).exists() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

impl Frames {
    pub fn new(name: String) -> Self {
        sweep_orphans(Path::new(SHM));
        Self {
            name: format!("phone.{}.{name}", std::process::id()),
            shown: 0,
            kept: VecDeque::new(),
        }
    }

    fn path(&self, n: u64) -> String {
        format!("{SHM}/{}-{n}", self.name)
    }

    fn put(&mut self, frame: &[u8]) -> std::io::Result<String> {
        while let Some(&(n, at)) = self.kept.front() {
            if self.kept.len() < KEEP || at.elapsed() < LINGER {
                break;
            }
            let _ = std::fs::remove_file(self.path(n));
            self.kept.pop_front();
        }

        self.shown += 1;
        std::fs::write(self.path(self.shown), frame)?;
        self.kept.push_back((self.shown, Instant::now()));
        Ok(format!("/{}-{}", self.name, self.shown))
    }
}

impl Drop for Frames {
    fn drop(&mut self) {
        for &(n, _) in &self.kept {
            let _ = std::fs::remove_file(self.path(n));
        }
    }
}

impl Output {
    pub fn new(shm: Option<String>, base64: bool) -> Self {
        match (shm, base64) {
            (Some(name), _) => Self::Shm(Frames::new(name)),
            (None, true) => Self::Base64,
            (None, false) => Self::Raw,
        }
    }

    fn write(&mut self, out: &mut impl Write, frame: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Raw => out.write_all(frame),
            Self::Base64 => writeln!(out, "{}", encode(frame)),
            Self::Shm(frames) => frames.put(frame).and_then(|name| writeln!(out, "{name}")),
        }?;
        out.flush()
    }
}

enum Fed {
    Closed,
    Stalled,
}

pub async fn stream(
    server: &Server,
    device: &Device,
    size: Size,
    mut output: Output,
) -> Result<()> {
    let mut frame = vec![0u8; (size.width * size.height * 3) as usize];
    let mut out = std::io::stdout().lock();

    while let Fed::Stalled = feed(server, device, size, &mut output, &mut out, &mut frame).await? {}

    Ok(())
}

async fn feed(
    server: &Server,
    device: &Device,
    size: Size,
    output: &mut Output,
    out: &mut impl Write,
    frame: &mut [u8],
) -> Result<Fed> {
    let mut source = spawned(server, device, size).await?;
    let mut h264 = source.stdout.take().context("the encoder gave no stdout")?;

    let mut decoder = Decoder::new().context("starting the h264 decoder")?;
    let mut pending = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    let mut full = Vec::new();
    let mut undecoded: Option<Instant> = None;
    let mut unsettled = false;

    loop {
        let whole = match timeout(wait(&pending, unsettled), h264.read(&mut chunk)).await {
            Ok(Ok(0) | Err(_)) => break,
            Ok(Ok(n)) => {
                pending.extend_from_slice(&chunk[..n]);
                last_start(&pending)
            }
            Err(_) if pending.is_empty() => {
                unsettled = false;
                if !settle(server, device, size, output, out, frame).await {
                    return Ok(Fed::Closed);
                }
                continue;
            }
            Err(_) => pending.len(),
        };

        if undecoded.get_or_insert_with(Instant::now).elapsed() > STALL {
            return Ok(Fed::Stalled);
        }

        if whole == 0 {
            continue;
        }

        if let Some((width, height)) = decode(&mut decoder, &pending[..whole], &mut full) {
            shrink(&full, width, height, frame, size);
            undecoded = None;
            unsettled = device.platform.is_adb();

            if output.write(out, frame).is_err() {
                return Ok(Fed::Closed);
            }
        }

        pending.drain(..whole);
    }

    bail!("the stream from {} ended", device.label)
}

fn wait(pending: &[u8], unsettled: bool) -> Duration {
    match (pending.is_empty(), unsettled) {
        (false, _) => PAUSE,
        (true, true) => SETTLE,
        (true, false) => Duration::MAX,
    }
}

async fn spawned(server: &Server, device: &Device, size: Size) -> Result<tokio::process::Child> {
    let mut source = source(server, device, size).await?;
    source
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    #[cfg(target_os = "linux")]
    let child = ur_taking_me_with_you::spawn_dying_with_parent_async(source);
    #[cfg(not(target_os = "linux"))]
    let child = source.spawn();

    child.context("starting the screen encoder")
}

fn decode(decoder: &mut Decoder, stream: &[u8], full: &mut Vec<u8>) -> Option<(usize, usize)> {
    let mut decoded = None;
    for unit in units(stream) {
        if let Ok(Some(yuv)) = decoder.decode(unit) {
            full.resize(yuv.rgb8_len(), 0);
            yuv.write_rgb8(full);
            decoded = Some(yuv.dimensions());
        }
    }
    decoded
}

async fn settle(
    server: &Server,
    device: &Device,
    size: Size,
    output: &mut Output,
    out: &mut impl Write,
    frame: &mut [u8],
) -> bool {
    let Some((rgb, width, height)) = settled(server, device).await else {
        return true;
    };
    shrink(&rgb, width, height, frame, size);
    output.write(out, frame).is_ok()
}

async fn settled(server: &Server, device: &Device) -> Option<(Vec<u8>, usize, usize)> {
    let png = capture(server, device, &Reporter::default()).await.ok()?;
    let rgb = image::load_from_memory(&png).ok()?.to_rgb8();
    let (width, height) = (rgb.width() as usize, rgb.height() as usize);
    Some((rgb.into_raw(), width, height))
}

async fn source(server: &Server, device: &Device, size: Size) -> Result<tokio::process::Command> {
    if device.platform == Platform::Simulator {
        let mut receiver = simctl::stream(&where_of(device), simctl::udid(device)?, ENCODED_WIDTH)?;
        receiver.stdin(Stdio::piped());

        return Ok(receiver);
    }

    if !device.platform.is_adb() {
        bail!("cannot stream {}", device.platform);
    }

    let serial = serial_of(server, device).await?;
    let encoded = encoded(size);

    let mut adb = tokio::process::Command::from(server.command());
    adb.args([
        "-s",
        &serial,
        "exec-out",
        "screenrecord",
        "--output-format=h264",
        "--time-limit",
        "0",
        "--bit-rate",
        "2000000",
        "--size",
        &format!("{}x{}", encoded.width, encoded.height),
        "-",
    ])
    .stdin(Stdio::null());

    Ok(adb)
}

fn last_start(stream: &[u8]) -> usize {
    let Some(at) = stream.windows(3).rposition(|w| w == [0, 0, 1]) else {
        return 0;
    };

    match at > 0 && stream[at - 1] == 0 {
        true => at - 1,
        false => at,
    }
}

fn units(stream: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut rest = stream;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = match rest.windows(3).skip(3).position(|w| w == [0, 0, 1]) {
            Some(at) => last_start(&rest[..at + 6]),
            None => rest.len(),
        };
        let (unit, tail) = rest.split_at(end);
        rest = tail;
        Some(unit)
    })
}

fn shrink(rgb: &[u8], width: usize, height: usize, into: &mut [u8], size: Size) {
    let (tw, th) = (size.width as usize, size.height as usize);

    for ty in 0..th {
        let (y0, y1) = span(ty, th, height);

        for tx in 0..tw {
            let (x0, x1) = span(tx, tw, width);
            let mut sum = [0u32; 3];

            for y in y0..y1 {
                for px in rgb[(y * width + x0) * 3..(y * width + x1) * 3].chunks_exact(3) {
                    sum.iter_mut()
                        .zip(px)
                        .for_each(|(s, c)| *s += u32::from(*c));
                }
            }

            let n = ((y1 - y0) * (x1 - x0)) as u32;
            let at = (ty * tw + tx) * 3;
            into[at..at + 3]
                .iter_mut()
                .zip(sum)
                .for_each(|(c, s)| *c = (s / n) as u8);
        }
    }
}

fn span(i: usize, cells: usize, pixels: usize) -> (usize, usize) {
    let start = (i * pixels / cells).min(pixels - 1);
    (start, ((i + 1) * pixels / cells).clamp(start + 1, pixels))
}

fn encoded(size: Size) -> Size {
    let even = |v: f64| ((v / 2.0).round() as u32).max(1) * 2;
    let height = f64::from(ENCODED_WIDTH) * f64::from(size.height) / f64::from(size.width);

    Size {
        width: ENCODED_WIDTH,
        height: even(height),
    }
}

pub fn encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));

        for i in 0..4 {
            match i <= chunk.len() {
                true => out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char),
                false => out.push('='),
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_phone_frames_name_an_owner() {
        assert_eq!(owner("phone.4242.emu-17-3-9"), Some(4242));
        assert_eq!(owner("phone.x.emu-1"), None);
        assert_eq!(owner("pulse-shm-4242"), None);
        assert_eq!(owner("emu-1790957709674-3-1"), None);
    }

    #[test]
    fn base64_pads_like_the_standard_alphabet() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(&[255, 0, 128, 7]), "/wCABw==");
    }

    #[test]
    fn the_encoder_keeps_the_asked_aspect_on_even_sides() {
        assert_eq!(
            encoded(Size {
                width: 40,
                height: 90
            }),
            Size {
                width: 432,
                height: 972
            }
        );
    }

    #[test]
    fn only_units_followed_by_another_start_are_whole() {
        assert_eq!(last_start(&[0, 0, 0, 1, 7, 9]), 0);
        assert_eq!(last_start(&[0, 0, 0, 1, 7, 9, 0, 0, 0, 1, 8]), 6);
        assert_eq!(last_start(&[0, 0, 1, 7, 9, 0, 0, 1, 8]), 5);
    }

    #[test]
    fn each_unit_keeps_its_start_code() {
        let stream = [0, 0, 0, 1, 7, 9, 0, 0, 1, 8, 0, 0, 0, 1, 5, 4];
        let units: Vec<&[u8]> = units(&stream).collect();
        assert_eq!(
            units,
            [&[0, 0, 0, 1, 7, 9][..], &[0, 0, 1, 8], &[0, 0, 0, 1, 5, 4]]
        );
    }

    #[test]
    fn shrinking_averages_each_block() {
        let rgb = [0, 0, 0, 200, 100, 50, 100, 50, 0, 100, 50, 250];
        let mut into = [0u8; 3];
        shrink(
            &rgb,
            2,
            2,
            &mut into,
            Size {
                width: 1,
                height: 1,
            },
        );
        assert_eq!(into, [100, 50, 75]);
    }
}
