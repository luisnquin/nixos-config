//! The half of `phone` that has to run on the Mac.
//!
//! adb has a server to point a client at and tunneld speaks HTTP, so those
//! devices can be driven from anywhere. CoreSimulator is launchd and mach ports
//! with no socket to forward, and reading or pressing a simulator needs private
//! frameworks that only load on macOS. So the controlling host runs this over
//! ssh, exactly the way it runs `adb` on a host that owns a handset.
//!
//! Everything is one call in and JSON out. Nodes come back in the shape the
//! controlling host already parses out of a `uiautomator` dump, so the matching
//! and ambiguity rules stay in one implementation.

mod accessibility;
mod error;
mod keys;
mod native;
mod node;

use std::io::Write as _;
use std::process::ExitCode;
use std::time::Duration;

use accessibility::interactive_accessibility_snapshot;
use anyhow::{anyhow, Result};
use keys::hid_for_char;
use native::bridge::NativeBridge;
use native::ffi;

const USAGE: &str = "\
usage: phone <verb> [args]

  devices               every simulator this host knows
  size <udid>           the panel in points and its scale
  snapshot <udid>       the elements that can be read or pressed
  tap <udid> <x> <y>    a touch, in points
  swipe <udid> <x1> <y1> <x2> <y2> [ms]
                        a drag, in points; both ends alike is a long press
  text <udid> <string>  one key event per character
  key <udid> <name>     a named key or a hardware button
  shot <udid>           a PNG on stdout

keys:    enter escape delete tab space up down left right
buttons: home app-switcher lock power side siri volume-up volume-down keyboard";

/// A drag is sent as a touch that moves, so it needs a rate: roughly a frame,
/// which is what a finger on a panel produces.
const STEP_MS: u64 = 16;
const DEFAULT_SWIPE_MS: u64 = 300;

/// USB HID keyboard usages, handed to SimulatorKit unchanged.
const NAMED_KEYS: &[(&str, u16)] = &[
    ("enter", 40),
    ("return", 40),
    ("escape", 41),
    ("delete", 42),
    ("backspace", 42),
    ("tab", 43),
    ("space", 44),
    ("right", 79),
    ("left", 80),
    ("down", 81),
    ("up", 82),
];

/// Names `pressHardwareButtonNamed:` answers to. It rejects anything else rather
/// than ignoring it, so the list is not enforced twice.
const BUTTONS: &[&str] = &[
    "home",
    "app-switcher",
    "lock",
    "power",
    "side",
    "siri",
    "volume-up",
    "volume-down",
    "keyboard",
    "apple-pay",
    "action",
    "mute",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("phone: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    let Some((verb, rest)) = args.split_first() else {
        println!("{USAGE}");
        return Ok(());
    };

    // SimulatorKit reaches the simulator through an NSApplication that a plain
    // CLI never stands up, and every call below fails without one.
    unsafe { ffi::xcw_native_initialize_app() };

    let bridge = NativeBridge;

    match verb.as_str() {
        "devices" => {
            let simulators = bridge.list_simulators()?;

            println!("{}", serde_json::to_string(&simulators)?);
        }
        "size" => {
            let size = bridge.display_size(udid(rest)?)?;

            println!("{}", serde_json::to_string(&size)?);
        }
        "snapshot" => {
            let tree = bridge.accessibility_snapshot(udid(rest)?, None)?;
            let tree = interactive_accessibility_snapshot(&tree);

            println!("{}", serde_json::to_string(&node::flatten(&tree))?);
        }
        "tap" => {
            let udid = udid(rest)?;
            let x = coordinate(rest, 1, "x")?;
            let y = coordinate(rest, 2, "y")?;

            // SimulatorKit takes a fraction of the panel and clamps rather than
            // rejects, so points handed straight through land in a corner.
            let size = bridge.display_size(udid)?;
            let x = x / size.width;
            let y = y / size.height;

            // Both phases ride one connection. Sent as two standalone calls the
            // reconnect between them stretches the touch into a long press, and
            // the device answers with a context menu instead of a tap.
            let session = bridge.create_input_session(udid)?;

            session.send_touch(x, y, "began")?;
            session.send_touch(x, y, "ended")?;
        }
        "swipe" => {
            let udid = udid(rest)?;
            let from = (coordinate(rest, 1, "x1")?, coordinate(rest, 2, "y1")?);
            let to = (coordinate(rest, 3, "x2")?, coordinate(rest, 4, "y2")?);

            let ms: u64 = match rest.get(5) {
                Some(value) => value.parse().map_err(|_| anyhow!("ms is not a number"))?,
                None => DEFAULT_SWIPE_MS,
            };

            let size = bridge.display_size(udid)?;
            let at = |(x, y): (f64, f64)| (x / size.width, y / size.height);

            // Every phase rides one connection, as a tap does: a reconnect
            // between them ends the gesture and starts another one. It is also
            // what makes a hold work — the touch is down for as long as the
            // moved events keep coming.
            let session = bridge.create_input_session(udid)?;
            let steps = (ms / STEP_MS).clamp(2, 120);
            let (x, y) = at(from);

            session.send_touch(x, y, "began")?;

            for step in 1..=steps {
                std::thread::sleep(Duration::from_millis(ms / steps));

                let travelled = step as f64 / steps as f64;
                let (x, y) = at((
                    from.0 + (to.0 - from.0) * travelled,
                    from.1 + (to.1 - from.1) * travelled,
                ));

                session.send_touch(x, y, "moved")?;
            }

            let (x, y) = at(to);

            session.send_touch(x, y, "ended")?;
        }
        "text" => {
            let udid = udid(rest)?;
            let text = rest.get(1).ok_or_else(|| anyhow!("text: no string"))?;

            // Refused whole rather than half sent, because a character with no
            // key would otherwise be dropped after the ones before it landed.
            let unsendable: Vec<char> = text
                .chars()
                .filter(|c| hid_for_char(*c).is_none())
                .collect();

            if !unsendable.is_empty() {
                return Err(anyhow!(
                    "cannot type {unsendable:?} — no key sends those characters"
                ));
            }

            for character in text.chars() {
                let (key, modifiers) =
                    hid_for_char(character).ok_or_else(|| anyhow!("no key sends {character:?}"))?;

                bridge.send_key(udid, key, modifiers)?;
            }
        }
        "key" => {
            let udid = udid(rest)?;
            let name = rest
                .get(1)
                .ok_or_else(|| anyhow!("key: no name"))?
                .to_lowercase();

            match NAMED_KEYS.iter().find(|(known, _)| *known == name) {
                Some((_, usage)) => bridge.send_key(udid, *usage, 0)?,
                None if BUTTONS.contains(&name.as_str()) => bridge.press_button(udid, &name, 60)?,
                None => return Err(anyhow!("unknown key: {name}")),
            }
        }
        "shot" => {
            let png = bridge.screenshot_png(udid(rest)?)?;

            std::io::stdout().write_all(&png)?;
        }
        other => return Err(anyhow!("unknown verb: {other}\n\n{USAGE}")),
    }

    Ok(())
}

fn udid(rest: &[String]) -> Result<&str> {
    rest.first()
        .map(String::as_str)
        .ok_or_else(|| anyhow!("no udid"))
}

fn coordinate(rest: &[String], at: usize, name: &str) -> Result<f64> {
    rest.get(at)
        .ok_or_else(|| anyhow!("no {name}"))?
        .parse()
        .map_err(|_| anyhow!("{name} is not a number"))
}
