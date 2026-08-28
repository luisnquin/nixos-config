//! A clip of the screen, and stills taken out of it.
//!
//! A screenshot answers what is on screen; it cannot answer what happened on
//! the way there. An animation that lands wrong, a splash that never clears, a
//! gesture that scrolled the inner list instead of the page — all of those look
//! identical before and after, and only differ in between.
//!
//! The video itself is unreadable to anything that reads text, so `--frames`
//! turns it into evenly spaced stills. That extraction happens here rather than
//! on the device's host: the clip has already crossed the link by then, and the
//! host holding a simulator is not required to have ffmpeg on it.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};

use crate::actions::{self, where_of, Shot};
use crate::adb::{self, Server};
use crate::connect::{attached_serial, Reporter};
use crate::model::{Device, Platform};
use crate::registry::state_dir;
use crate::{simctl, ssh};

/// `screenrecord` refuses anything longer outright, and a clip that long is
/// past the point where stills are the wrong tool anyway.
pub const LONGEST: u32 = 180;

/// What to take, and what to leave behind.
#[derive(Clone, Debug)]
pub struct Take {
    pub seconds: u32,
    /// Stills to pull out of the clip.
    pub frames: Option<Frames>,
    /// Where the clip goes. Without one it lands under the state directory and
    /// the path is printed, so a caller that only wanted the frames does not
    /// have to invent a name for a file it will not open.
    pub out: Option<PathBuf>,
    /// Applied to the extracted stills, not to the clip.
    pub shot: Shot,
}

/// The clip, then whatever stills were asked of it. Paths, in the order they
/// should be read: the video first, then the frames in time order.
pub async fn record(
    server: &Server,
    device: &Device,
    take: &Take,
    rep: &Reporter,
) -> Result<Vec<PathBuf>> {
    if take.seconds == 0 || take.seconds > LONGEST {
        bail!("a clip runs between 1 and {LONGEST} seconds");
    }

    let mp4 = match &take.out {
        Some(path) => path.clone(),
        None => scratch(&format!("{}.mp4", stem(device))),
    };

    if let Some(dir) = mp4.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
    }

    rep.try_(format!("recording {}s", take.seconds));

    let clip = match device.platform {
        Platform::Ios => bail!("cannot record an iPhone from here"),
        Platform::Simulator => from_simulator(device, take.seconds).await?,
        _ => from_android(server, device, take.seconds).await?,
    };

    if clip.is_empty() {
        bail!("the recording came back empty");
    }

    std::fs::write(&mp4, &clip).with_context(|| format!("writing {}", mp4.display()))?;
    rep.done(format!("{} ({} KiB)", mp4.display(), clip.len() / 1024));

    let mut out = vec![mp4.clone()];

    if let Some(want) = take.frames {
        out.extend(extract(&mp4, want, &take.shot, rep).await?);
    }

    Ok(out)
}

fn stem(device: &Device) -> String {
    device
        .label
        .chars()
        .map(|c| match c.is_ascii_alphanumeric() {
            true => c.to_ascii_lowercase(),
            false => '-',
        })
        .collect()
}

fn scratch(name: &str) -> PathBuf {
    state_dir().join("clips").join(name)
}

/// `screenrecord` writes to the device and nowhere else, so the clip is pulled
/// back afterwards rather than streamed. `exec-out` keeps the bytes binary —
/// the same reason `shot` uses it.
async fn from_android(server: &Server, device: &Device, seconds: u32) -> Result<Vec<u8>> {
    let serial = attached_serial(server, device)
        .await
        .ok_or_else(|| anyhow!("{} is not attached", device.label))?;

    // a path under the app-visible part of storage, so the pull needs no root
    let on_device = "/sdcard/phone-record.mp4";

    let out = adb::run_timeout(
        server,
        &[
            "-s",
            &serial,
            "shell",
            &format!("screenrecord --time-limit {seconds} {on_device}"),
        ],
        // the device stops on its own at `seconds`; the slack is the muxer
        // writing the file out afterwards
        Duration::from_secs(u64::from(seconds) + 30),
    )
    .await?;

    if !out.ok() {
        let said = out.stderr.trim();

        // an emulator without the encoder says so here rather than at boot
        bail!(match said.is_empty() {
            true => "screenrecord failed".to_string(),
            false => said.to_string(),
        });
    }

    let (ok, bytes) = adb::run_bytes(
        server,
        &["-s", &serial, "exec-out", &format!("cat {on_device}")],
    )
    .await?;

    // best effort: a clip left behind costs storage, not correctness
    let _ = adb::run_timeout(
        server,
        &["-s", &serial, "shell", &format!("rm -f {on_device}")],
        Duration::from_secs(10),
    )
    .await;

    if !ok {
        bail!("could not read the clip back off the device");
    }

    Ok(bytes)
}

/// `simctl io recordVideo` runs until it is interrupted, so the length is set
/// by when the signal arrives rather than by a flag. INT rather than TERM: it
/// is the one that makes simctl finish the file instead of abandoning it.
async fn from_simulator(device: &Device, seconds: u32) -> Result<Vec<u8>> {
    const SCRIPT: &str = r#"clip="$(mktemp -t phone-record).mp4"
xcrun simctl io "$1" recordVideo --codec h264 --force "$clip" >/dev/null 2>&1 &
pid=$!
sleep "$2"
kill -INT "$pid" 2>/dev/null
wait "$pid" 2>/dev/null
cat "$clip"
rm -f "$clip""#;

    let udid = simctl::udid(device)?;

    let bytes = ssh::output(
        where_of(device).run(SCRIPT, &[udid, &seconds.to_string()]),
        Duration::from_secs(u64::from(seconds) + 60),
    )
    .await?;

    Ok(bytes)
}

/// Evenly spaced across the clip, first and last inclusive. Seeking to each
/// timestamp separately rather than decoding the whole clip once: a 30s capture
/// is a thousand frames and four of them are wanted.
async fn extract(mp4: &Path, want: Frames, shot: &Shot, rep: &Reporter) -> Result<Vec<PathBuf>> {
    let clip = probe(mp4).await?;
    let stem = mp4.with_extension("");
    let at = moments(mp4, &clip, want, rep).await?;

    let mut out = Vec::new();
    let mut missed = 0;

    for (n, at) in at.iter().enumerate() {
        let frame = PathBuf::from(format!("{}-{}.png", stem.display(), n + 1));

        match still(mp4, *at, &frame, shot).await? {
            Some(frame) => {
                rep.done(format!("{} at {at:.1}s", frame.display()));
                out.push(frame);
            }
            // one still that cannot be taken is not a clip that failed. The
            // video and every other frame are still on disk and still worth
            // reading, so this is said and the run goes on.
            None => {
                missed += 1;
                rep.fail(format!("no frame at {at:.1}s"));
            }
        }
    }

    if out.is_empty() {
        bail!("the clip yielded no frames at all");
    }

    if missed > 0 {
        rep.note(format!(
            "{} of {} stills could not be taken; the clip has them all",
            missed,
            at.len()
        ));
    }

    Ok(out)
}

/// One still, or `None` where the clip holds no image at that instant. ffmpeg
/// answers a seek past the last packet with no file and no complaint, so the
/// file is what is checked rather than the status.
async fn still(mp4: &Path, at: f64, frame: &Path, shot: &Shot) -> Result<Option<PathBuf>> {
    let ok = tokio::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-ss"])
        .arg(format!("{at:.3}"))
        .arg("-i")
        .arg(mp4)
        .args(["-frames:v", "1"])
        .arg(frame)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| anyhow!("running ffmpeg: {e}"))?;

    if !ok.success() || !frame.exists() {
        return Ok(None);
    }

    // the same crop, scale and encoding a `shot` would have had, so a frame
    // costs a reader what a screenshot costs
    let png = std::fs::read(frame)?;

    match actions::render(png, shot)? {
        rendered if shot.jpeg.is_some() => {
            let jpg = frame.with_extension("jpg");

            std::fs::write(&jpg, rendered)?;
            std::fs::remove_file(frame)?;

            Ok(Some(jpg))
        }
        rendered => {
            std::fs::write(frame, rendered)?;

            Ok(Some(frame.to_path_buf()))
        }
    }
}

/// How many stills, and chosen how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frames {
    /// Evenly spaced across the clip.
    Count(u8),
    /// Wherever the picture actually changed.
    Changed,
}

/// More than this and the stills stop being cheaper to read than the clip. A
/// scrolling list changes on every frame, and none of those changes is the one
/// the clip was taken for.
const MOST: usize = 12;

/// Below this two stills are the same picture twice: an animation is a run of
/// frames that each differ slightly from the last, and a scene cut inside one
/// is still one event.
const APART: f64 = 0.15;

/// The moments the picture changed, anchored by the first and last frame so the
/// before and after are always in the set.
///
/// Even spacing is the right default and the wrong sampling for a route change:
/// a tap at 1.0s whose animation is over by 1.5s leaves four evenly spaced
/// stills holding one old screen and three identical new ones, with the
/// transition in the half second the spacing stepped over.
async fn changed(mp4: &Path, clip: &Clip, rep: &Reporter) -> Result<Vec<f64>> {
    let out = tokio::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "info", "-i"])
        .arg(mp4)
        // `scene` is how much of the picture differs from the frame before it;
        // showinfo then prints what survived, timestamps and all, to stderr
        .args(["-vf", "select='gt(scene,0.08)',showinfo", "-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .await
        .map_err(|e| anyhow!("running ffmpeg: {e}"))?;

    let found = scenes(&String::from_utf8_lossy(&out.stderr));
    let mut at = vec![0.0];

    at.extend(found.iter().copied().filter(|t| *t > 0.0 && *t < clip.last));
    at.push(clip.last);
    at.dedup_by(|b, a| *b - *a < APART);

    let kept = capped(at);

    rep.note(match (found.len(), kept.len()) {
        (0, _) => "nothing changed in the clip; the first and last frame are it".to_string(),
        (found, keeping) if found + 2 > keeping => {
            format!("{found} changes, keeping {keeping}")
        }
        (found, _) => format!(
            "{found} change{}",
            match found {
                1 => "",
                _ => "s",
            }
        ),
    });

    // where to look first, which is worth knowing before spending anything on
    // reading an image: the longest stretch nothing was captured in
    if let Some((after, gap)) = widest(&kept) {
        rep.note(format!("biggest gap {gap:.1}s after {after:.1}s"));
    }

    Ok(kept)
}

/// `showinfo` writes one line per frame it was handed, and the time is one
/// field among many on it.
fn scenes(said: &str) -> Vec<f64> {
    said.lines()
        .filter(|l| l.contains("showinfo"))
        .filter_map(|l| l.split("pts_time:").nth(1))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter_map(|t| t.parse::<f64>().ok())
        .collect()
}

/// Thins a set too big to read, keeping both ends and spreading what is left
/// over the middle. Dropping the tail instead would drop the end state, which
/// is half of what the stills are for.
fn capped(at: Vec<f64>) -> Vec<f64> {
    if at.len() <= MOST {
        return at;
    }

    let middle = &at[1..at.len() - 1];
    let keep = MOST - 2;

    let mut out = vec![at[0]];

    out.extend((0..keep).map(|n| middle[n * (middle.len() - 1) / (keep - 1)]));
    out.push(at[at.len() - 1]);
    out.dedup();

    out
}

/// The longest stretch between two kept stills, and when it starts.
fn widest(at: &[f64]) -> Option<(f64, f64)> {
    at.windows(2)
        .map(|w| (w[0], w[1] - w[0]))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, gap)| *gap > 0.0)
}

/// The instants to take stills at, in time order.
async fn moments(mp4: &Path, clip: &Clip, want: Frames, rep: &Reporter) -> Result<Vec<f64>> {
    match want {
        Frames::Changed => changed(mp4, clip, rep).await,
        Frames::Count(count) => Ok(spread(clip, count, rep)),
    }
}

/// Evenly spaced, first and last inclusive: the point of the stills is the
/// start state and the end state with the change between them, so both ends
/// have to be in the set.
fn spread(clip: &Clip, count: u8, rep: &Reporter) -> Vec<f64> {
    // a simulator encodes on damage rather than on a clock, so a screen that
    // did not move for six seconds is a clip one frame long. Asking it for four
    // stills is asking for three that do not exist.
    let count = match clip.held.filter(|held| *held < u32::from(count)) {
        Some(held) => {
            rep.note(format!(
                "the clip holds {held} frame{} — nothing moved for most of it",
                match held {
                    1 => "",
                    _ => "s",
                }
            ));

            held.max(1) as u8
        }
        None => count,
    };

    (0..count)
        .map(|n| {
            let at = match count {
                1 => clip.span / 2.0,
                _ => clip.span * f64::from(n) / f64::from(count - 1),
            };

            at.min(clip.last).max(0.0)
        })
        .collect()
}

/// What a clip turns out to be, which is not what was asked for:
/// `screenrecord` stops early on a device under load, and a simulator writes a
/// frame only when the screen changes.
struct Clip {
    /// How long it runs, by the container's own reckoning.
    span: f64,
    /// How many frames it holds. What a container chose to record and some do
    /// not, so `None` rather than a guess.
    held: Option<u32>,
    /// When the last frame starts. Read rather than derived: dividing the span
    /// by the count assumes the frames are evenly spaced, and on a simulator
    /// they are spaced by whenever the screen moved. A clip whose last second
    /// is still holds its final frame a second before it ends, and a seek into
    /// the gap behind it lands on nothing at all.
    last: f64,
}

async fn probe(mp4: &Path) -> Result<Clip> {
    let out = ffprobe(
        mp4,
        &[
            "-show_entries",
            "format=duration",
            "-show_entries",
            "stream=nb_frames",
        ],
    )
    .await?;

    let (span, held) =
        read(&out).ok_or_else(|| anyhow!("{} is not a video ffprobe can read", mp4.display()))?;

    // the times come back in the order the packets are stored, which for a
    // stream carrying B-frames is not the order they are shown in
    let times = ffprobe(mp4, &["-show_entries", "packet=pts_time"]).await?;
    let last = times
        .lines()
        .filter_map(|l| l.trim().parse::<f64>().ok())
        .fold(f64::NAN, f64::max);

    Ok(Clip {
        span,
        held,
        // a container that lists no packet times leaves only the span to go on
        last: match last.is_finite() {
            true => last,
            false => (span - 0.05).max(0.0),
        },
    })
}

async fn ffprobe(mp4: &Path, entries: &[&str]) -> Result<String> {
    let out = tokio::process::Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0"])
        .args(entries)
        .args(["-of", "csv=p=0"])
        .arg(mp4)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| anyhow!("running ffprobe: {e}"))?;

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// ffprobe prints the stream's line first and the format's second, one value to
/// a line, and writes `N/A` for a count it does not have.
fn read(said: &str) -> Option<(f64, Option<u32>)> {
    let mut lines = said.lines().map(str::trim).filter(|l| !l.is_empty());
    let frames = lines.next()?;
    let span: f64 = lines.next()?.parse().ok()?;

    Some((span, frames.parse().ok())).filter(|(span, _)| *span > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::connect::Step;

    fn watched() -> (Reporter, tokio::sync::mpsc::UnboundedReceiver<Step>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        (Reporter::new(tx), rx)
    }

    fn clip(span: f64, held: Option<u32>, last: f64) -> Clip {
        Clip { span, held, last }
    }

    fn spaced(count: u8, span: f64) -> Vec<f64> {
        spread(&clip(span, None, span), count, &watched().0)
    }

    /// The point of the stills is the start state and the end state with the
    /// change between them, so both ends have to be in the set.
    #[test]
    fn frames_span_the_whole_clip_end_to_end() {
        assert_eq!(spaced(3, 6.0), [0.0, 3.0, 6.0]);
        assert_eq!(spaced(2, 5.0), [0.0, 5.0]);
    }

    /// One frame has no span to divide, and the interesting moment of a short
    /// clip is not its first frame.
    #[test]
    fn a_single_frame_is_taken_from_the_middle() {
        assert_eq!(spaced(1, 6.0), [3.0]);
    }

    /// A clip whose last second is still holds its final frame a second before
    /// it ends. Asking for the instant after that asks for no image at all.
    #[test]
    fn no_still_is_asked_for_past_the_last_frame() {
        let taken = spread(&clip(5.0, None, 3.2), 3, &watched().0);

        assert_eq!(taken, [0.0, 2.5, 3.2]);
    }

    #[test]
    fn reads_the_frame_count_and_the_length_off_ffprobe() {
        assert_eq!(read("1\n0.066667\n"), Some((0.066667, Some(1))));
        assert_eq!(read("90\n6.0\n"), Some((6.0, Some(90))));
    }

    /// Not every container records a frame count, and a clip is still a clip
    /// without one — only the trimming of `--frames` depends on it.
    #[test]
    fn a_missing_frame_count_is_not_a_broken_clip() {
        assert_eq!(read("N/A\n6.0\n"), Some((6.0, None)));
        assert_eq!(read(""), None);
        assert_eq!(read("1\n0\n"), None);
    }

    /// showinfo prints one line per frame that survived the filter, and the
    /// time is one field among a dozen on it.
    #[test]
    fn reads_the_moments_a_picture_changed_off_showinfo() {
        let said = "\
[Parsed_showinfo_1 @ 0x600] n:0 pts:0 pts_time:0 duration:1 fmt:rgba
frame= 2 fps=0.0 q=-0.0 size=N/A time=00:00:05.00 bitrate=N/A
[Parsed_showinfo_1 @ 0x600] n:1 pts:63 pts_time:1.05 duration:1 fmt:rgba";

        assert_eq!(scenes(said), [0.0, 1.05]);
    }

    /// A scrolling list changes on every frame, and sixty stills is not an
    /// answer. Both ends survive the thinning: the end state is half of what
    /// the stills are for.
    #[test]
    fn a_clip_that_never_stops_moving_is_thinned_to_something_readable() {
        let many: Vec<f64> = (0..60).map(f64::from).collect();
        let kept = capped(many);

        assert_eq!(kept.len(), MOST);
        assert_eq!(kept.first(), Some(&0.0));
        assert_eq!(kept.last(), Some(&59.0));
    }

    #[test]
    fn a_set_small_enough_to_read_is_left_alone() {
        assert_eq!(capped(vec![0.0, 1.0, 2.0]), [0.0, 1.0, 2.0]);
    }

    /// Where to look first, which is worth knowing before spending anything on
    /// reading an image.
    #[test]
    fn names_the_longest_stretch_nothing_was_captured_in() {
        assert_eq!(widest(&[0.0, 0.5, 3.5, 4.0]), Some((0.5, 3.0)));
        assert_eq!(widest(&[1.0]), None);
    }

    /// The label becomes a filename, and a simulator's is `iPhone 17 Pro Max`.
    #[test]
    fn a_device_name_with_spaces_makes_one_word() {
        let mut device = Device::new("x", "iPhone 17 Pro Max", Platform::Simulator);
        device.label = "iPhone 17 Pro Max".into();

        assert_eq!(stem(&device), "iphone-17-pro-max");
    }
}
