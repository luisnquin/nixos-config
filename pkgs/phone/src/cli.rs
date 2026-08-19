use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "phone",
    about = "drive the handsets, emulators and iPhone on this desk",
    long_about = "Run without a command to open the device browser.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Device to act on; also read from PHONE_TARGET. The commands that take it
    /// positionally accept either.
    #[arg(short, long, global = true)]
    pub target: Option<String>,

    /// Press X,Y before reading or acting, to pull focus to the app meant
    /// (split screen: the dump and every key follow whichever window has it)
    #[arg(long, global = true, value_parser = parse_point)]
    pub focus: Option<(i32, i32)>,
}

/// How far a directional swipe travels when nothing says otherwise. Named so
/// that dispatch can tell "the default" from a figure the caller chose.
pub const DEFAULT_AMOUNT: f64 = 0.6;

#[derive(Subcommand)]
pub enum Command {
    /// List every reachable and remembered device
    Devices {
        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },

    /// Bring up a transport to a device, trying history before discovery
    Connect {
        #[arg(id = "device")]
        target: Option<String>,

        /// Skip the tailnet port sweep
        #[arg(long)]
        no_sweep: bool,

        /// Ports to sweep, as START-END
        #[arg(long, value_parser = parse_range)]
        range: Option<RangeInclusive<u16>>,

        /// Concurrent probes during the sweep
        #[arg(long, default_value_t = 512)]
        concurrency: usize,
    },

    /// Drop a wireless transport
    Disconnect {
        #[arg(id = "device")]
        target: Option<String>,

        /// Drop every wireless transport
        #[arg(long)]
        all: bool,
    },

    /// Pair with a device that is in wireless-debugging pairing mode
    Pair {
        /// Six digit code from the pairing dialog
        code: String,

        /// host:port from the dialog; discovered over mDNS when omitted
        #[arg(long)]
        addr: Option<String>,
    },

    /// Restart adbd on a fixed port so later reconnects skip discovery
    Pin {
        #[arg(id = "device")]
        target: Option<String>,

        #[arg(long, default_value_t = 5555)]
        port: u16,
    },

    /// Set the device other commands target by default
    Use {
        #[arg(id = "device")]
        target: Option<String>,
    },

    /// Drop a device from the registry
    Forget {
        #[arg(id = "device")]
        target: String,
    },

    /// Screenshot to the clipboard
    Shot {
        #[arg(id = "device")]
        target: Option<String>,

        /// Write to FILE instead of the clipboard; "-" writes the image to stdout
        #[arg(short, long)]
        out: Option<String>,

        /// Keep only part of the frame: an element by text, @index or X,Y,W,H
        #[arg(long)]
        crop: Option<String>,

        /// Room to leave around a cropped element, in the coordinate space
        #[arg(long, default_value_t = 24)]
        pad: i32,

        /// Resize by this factor, 1.0 being the panel's own resolution
        #[arg(long, value_parser = parse_scale)]
        scale: Option<f64>,

        /// Encode as JPEG at this quality rather than PNG
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        jpeg: Option<u8>,

        /// Capture until the screen stops changing
        #[arg(long)]
        settle: bool,
    },

    /// The panel, in the space taps and element bounds use
    Size {
        #[arg(id = "device")]
        target: Option<String>,

        /// Emit JSON instead of a line
        #[arg(long)]
        json: bool,
    },

    /// Stream logs for a package name or bundle id
    Logs { app: String },

    /// scrcpy mirror
    Mirror {
        #[arg(id = "device")]
        target: Option<String>,
    },

    /// Install an apk, or a .app bundle on a simulator
    Install { apk: PathBuf },

    /// Choose which ssh hosts to survey for devices
    Hosts {
        #[command(subcommand)]
        action: Option<HostAction>,
    },

    /// List what is on screen, as elements that can be named
    Snapshot {
        #[arg(id = "device")]
        target: Option<String>,

        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },

    /// Press an element, named by its text, description, @index or X,Y
    Tap { what: String },

    /// Hold an element down, named as `tap` names one
    Press {
        what: String,

        /// How long to keep the touch down
        #[arg(long, default_value = "800ms", value_parser = parse_duration)]
        hold: Duration,
    },

    /// Drag between two points, or scroll the screen a direction
    Swipe {
        /// X,Y, an element, or a direction (up, down, left, right)
        from: String,

        /// X,Y or an element; left out when the first is a direction
        to: Option<String>,

        /// How long the drag takes; a slow one scrolls, a fast one flings
        #[arg(long, default_value = "300ms", value_parser = parse_duration)]
        duration: Duration,

        /// How much of the panel a directional swipe crosses
        #[arg(long, default_value_t = DEFAULT_AMOUNT)]
        amount: f64,
    },

    /// Block until an element appears, or stop waiting and fail
    Wait {
        what: String,

        /// Wait for it to leave the screen instead
        #[arg(long)]
        gone: bool,

        #[arg(long, default_value = "15s", value_parser = parse_duration)]
        timeout: Duration,
    },

    /// Type into whatever holds focus
    Type { text: String },

    /// Send a key, by keycode name (back, home, enter, tab…)
    Key { name: String },

    /// Run several screen verbs against one device, surveying once
    Do {
        /// Each a whole command, quoted: `phone do "tap Login" "wait Inbox"`
        #[arg(required = true)]
        steps: Vec<String>,
    },

    /// Start a simulator or emulator and wait until it can be driven
    Boot {
        #[arg(id = "device")]
        target: Option<String>,

        /// Give up if it is still not usable by then
        #[arg(long, default_value = "180s", value_parser = parse_duration)]
        timeout: Duration,
    },

    /// Stop a running simulator or emulator
    Shutdown {
        #[arg(id = "device")]
        target: Option<String>,
    },

    /// Check the tools and daemons this depends on
    Doctor,

    /// Print a shell completion script
    Completions { shell: Shell },
}

impl Command {
    /// Whether this verb acts on one device's screen, and the device it named
    /// positionally. Those verbs share a resolved device, which is what lets a
    /// sequence of them survey once instead of once each.
    pub fn on_screen(&self) -> Option<Option<&str>> {
        let positional = match self {
            Command::Shot { target, .. }
            | Command::Size { target, .. }
            | Command::Snapshot { target, .. } => target.as_deref(),

            Command::Tap { .. }
            | Command::Press { .. }
            | Command::Swipe { .. }
            | Command::Wait { .. }
            | Command::Type { .. }
            | Command::Key { .. }
            | Command::Do { .. } => None,

            _ => return None,
        };

        Some(positional)
    }
}

#[derive(Subcommand)]
pub enum HostAction {
    /// Survey this host's devices from now on, probing what it can drive
    Enable { name: String },

    /// Stop surveying it
    Disable { name: String },
}

#[derive(Copy, Clone, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl From<Shell> for clap_complete::Shell {
    fn from(s: Shell) -> Self {
        match s {
            Shell::Bash => clap_complete::Shell::Bash,
            Shell::Zsh => clap_complete::Shell::Zsh,
            Shell::Fish => clap_complete::Shell::Fish,
        }
    }
}

pub fn parse_point(s: &str) -> Result<(i32, i32), String> {
    let (x, y) = s
        .split_once(',')
        .ok_or_else(|| "expected X,Y".to_string())?;

    Ok((
        x.trim().parse().map_err(|_| "bad x")?,
        y.trim().parse().map_err(|_| "bad y")?,
    ))
}

/// A number with a unit, because a bare one reads as either: `--hold 800` is
/// milliseconds to anyone who has used `input swipe` and seconds to anyone who
/// has used `sleep`.
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let (value, unit) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(s.len()),
    );

    let value: f64 = value.parse().map_err(|_| format!("{s} is not a time"))?;

    let seconds = match unit.trim() {
        "ms" => value / 1000.0,
        "s" | "" => value,
        "m" => value * 60.0,
        other => return Err(format!("unknown unit '{other}' (try ms, s, m)")),
    };

    Ok(Duration::from_secs_f64(seconds))
}

/// A factor, not a percentage or a pixel count. The ceiling is there because
/// the frame is decoded in memory: 1080x2400 at 10x is 260 megapixels.
fn parse_scale(s: &str) -> Result<f64, String> {
    let by: f64 = s
        .trim()
        .parse()
        .map_err(|_| format!("{s} is not a number"))?;

    if by <= 0.0 || by > 4.0 {
        return Err(format!("{by} is out of range (0 < scale <= 4)"));
    }

    Ok(by)
}

fn parse_range(s: &str) -> Result<RangeInclusive<u16>, String> {
    let (start, end) = s
        .split_once('-')
        .ok_or_else(|| "expected START-END".to_string())?;

    let start: u16 = start.trim().parse().map_err(|_| "bad start port")?;
    let end: u16 = end.trim().parse().map_err(|_| "bad end port")?;

    if start > end {
        return Err("start port is above end port".into());
    }

    Ok(start..=end)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A subcommand that names an argument the same as a global one silently
    /// takes it over, and the global stops reaching that subcommand at all.
    #[test]
    fn a_subcommand_does_not_shadow_a_global_flag() {
        use clap::CommandFactory;
        Cli::command().debug_assert();

        let cli = Cli::try_parse_from(["phone", "size", "-t", "faraday"]).unwrap();
        assert_eq!(cli.target.as_deref(), Some("faraday"));
    }

    #[test]
    fn a_time_carries_the_unit_it_is_in() {
        assert_eq!(parse_duration("800ms").unwrap(), Duration::from_millis(800));
        assert_eq!(parse_duration("15s").unwrap(), Duration::from_secs(15));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(
            parse_duration("15").unwrap(),
            Duration::from_secs(15),
            "a bare number is seconds, as everywhere else a timeout is typed"
        );

        assert!(parse_duration("soon").is_err());
        assert!(parse_duration("15 fortnights").is_err());
    }
}
