use std::ops::RangeInclusive;
use std::path::PathBuf;

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
}

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
        target: Option<String>,

        #[arg(long, default_value_t = 5555)]
        port: u16,
    },

    /// Set the device other commands target by default
    Use { target: Option<String> },

    /// Drop a device from the registry
    Forget { target: String },

    /// Screenshot to the clipboard
    Shot {
        target: Option<String>,

        /// Write to FILE instead of the clipboard; "-" writes PNG to stdout
        #[arg(short, long)]
        out: Option<String>,
    },

    /// Stream logs for a package name or bundle id
    Logs {
        /// Device, or the app when only one argument is given
        first: String,
        second: Option<String>,
    },

    /// scrcpy mirror
    Mirror { target: Option<String> },

    /// Install an apk, or a .app bundle on a simulator
    Install {
        apk: PathBuf,

        #[arg(long)]
        target: Option<String>,
    },

    /// Choose which ssh hosts to survey for devices
    Hosts {
        #[command(subcommand)]
        action: Option<HostAction>,
    },

    /// Check the tools and daemons this depends on
    Doctor,

    /// Print a shell completion script
    Completions { shell: Shell },
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
