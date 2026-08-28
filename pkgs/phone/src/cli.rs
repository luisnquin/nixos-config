use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "phone",
    about = "drive the handsets, emulators and iPhone on this desk",
    long_about = "Run without a command to open the device browser.",
    after_help = OVERVIEW,
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

/// Shown under `--help`. An agent that arrives here with no other
/// documentation has to be able to get from it to a working command, so it
/// describes the loop the verbs are meant to be used in rather than listing
/// them again.
const OVERVIEW: &str = r#"How this is meant to be used

  In a project with a phone.toml, one command puts everything where that file
  says it should be: devices booted, ports forwarded, a build no older than the
  sources installed, the app in front.

    phone up                 # start here; safe to repeat, and cheap when nothing moved
    phone status             # what drifted; exits non-zero when anything has

  Then drive it. A screen cannot be acted on until it has been read, so the loop
  is: read what is on it, press something by the name you just read, wait for the
  result, read again.

    phone snapshot           # read: every element on screen, by name
    phone tap "Log in"       # act: by a name that came from the snapshot
    phone wait Dashboard     # let the screen catch up before reading again
    phone shot --crop @2     # look, for anything text cannot describe

  Steps known in advance belong in one `do`, which runs them against one device:

    phone do "tap 'Log in'" "wait Dashboard" "shot --crop @2"

The commands

  project  up down status
  screen   snapshot shot size tap press swipe type key wait do
  device   list connect disconnect pair pin use forget boot shutdown reverse
  app      install launch stop open logs
  host     list enable disable
  this     mirror record doctor

What to know before scripting it

  Reading the screen costs tokens: a full frame is roughly 1500, while a
  --crop of one control, or --scale 0.3 --jpeg 60 of the whole thing, is a
  fraction of that. `snapshot` is text and usually answers the question anyway.

  What costs more than any single read is reading twice to find out whether the
  first act landed. `wait <name>` and `shot --settle` are the answer to that:
  both return once the screen has caught up, so the next read is the only read.

Naming things

  -t <name> targets one command and PHONE_TARGET a whole shell; both beat the
  default set by `phone device use`, and that beats the `default` a project's
  phone.toml names. A name matches on text, model, host or alias, and an
  ambiguous one is refused with the candidates listed rather than guessed at.

  @index numbers the rows of one snapshot only. Two commands are two dumps and
  the screen may have moved between them, so name elements by their text unless
  the index came from the command immediately before.

  A snapshot row printed as <View> or <EditText> has no name of its own — that
  is its class, shown in angle brackets because nothing will match on it. Reach
  those by the @index beside them.

Every verb carries its own examples: phone help <verb>."#;

/// How far a directional swipe travels when nothing says otherwise. Named so
/// that dispatch can tell "the default" from a figure the caller chose.
pub const DEFAULT_AMOUNT: f64 = 0.6;

#[derive(Subcommand)]
pub enum Command {
    // the project: everything phone.toml declares, brought to where it says
    /// Bring this project's devices to the state phone.toml declares
    #[command(after_help = r#"Examples:
  phone up
  phone up --profile e2e
  phone up --rebuild
  phone up --timeout 5m

Reads the nearest phone.toml, works out what is missing, and does only that:
boots what is off, attaches to it, forwards the ports it declares, writes the
settings it declares, builds when the sources have moved, and starts the app.

Safe to repeat, and that is the point. A second run against a converged project
opens the app and stops — it does not rebuild. A build happens when the project's
own freshness command prints something different from last time, or when the app
is not on the device at all, which is what makes a wiped emulator rebuild without
being told to. `--rebuild` forces it when the check is right and the answer is
still wrong.

Everything declared runs on the host phone.toml names, in the directory it names,
so a build and a bundler live wherever the sdk is rather than on this machine.
`phone status` says what is not there yet without changing any of it.

`--profile` narrows a run to a named subset of the declared devices. Without one
every declared device is converged, which is what makes `phone down` the exact
inverse of `phone up`.

Devices on different platforms converge at once rather than in turn, so an iPhone
does not wait out an android build. Two devices that would run the same build take
turns, since they would run it in the same directory. While more than one is going
each line of output says which device it came from."#)]
    Up {
        /// Only the devices this profile names
        #[arg(long)]
        profile: Option<String>,

        /// Run the build steps whatever the freshness checks say
        #[arg(long)]
        rebuild: bool,

        /// Give up on a device that is still not usable by then
        #[arg(long, default_value = "180s", value_parser = parse_duration)]
        timeout: Duration,
    },
    /// Stop what `up` started, leaving handsets alone
    #[command(after_help = r#"Examples:
  phone down

Stops the bundler, drops the forwards and shuts down the emulators and
simulators this project declares. A handset is left running: it was on before
`up` ran and `up` did not turn it on.

Every declared device, and no `--profile` to narrow it. A bare `up` brings all of
them up, so a teardown that took only some down would leave the rest as the
strays the next `status` complains about. To stop one device and nothing else,
name it: `phone device shutdown NAME`.

What is installed on a device stays installed, so the next `up` boots it and
goes straight to the app."#)]
    Down,
    /// Say what this project declares and what is actually there
    #[command(after_help = r#"Examples:
  phone status
  phone status --json
  phone status --profile e2e

Reads and changes nothing. Exits 0 when everything declared is where it was
declared to be and non-zero when anything has drifted, which is what lets a test
script gate on one command: `phone status || phone up`.

A `!` in the first column marks the rows that differ, and a device running that
the manifest never declared is listed under them."#)]
    Status {
        /// Only the devices this profile names
        #[arg(long)]
        profile: Option<String>,

        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },

    // the screen: read it, then act on what the reading named
    /// List what is on screen, as elements that can be named
    #[command(after_help = r#"Examples:
  phone snapshot
  phone snapshot --json
  phone --focus 297,1971 snapshot  # pull focus first, in split screen

The cheapest way to see a screen, and the one that gives names to press. Empty
on anything drawn rather than laid out — a game, a canvas, a video — and there
`shot` plus a coordinate is the only way through."#)]
    Snapshot {
        #[arg(id = "device")]
        target: Option<String>,

        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Screenshot the screen, or one control out of it
    #[command(after_help = r#"Examples:
  phone shot                         # PNG to the clipboard
  phone shot -o /tmp/s.png           # to a file; "-" writes the image to stdout
  phone shot --crop "Log in"         # just that control, padded
  phone shot --crop "Log in" --expand 1  # the row or card it sits in
  phone shot --crop @12 --pad 40     # the element snapshot numbered 12
  phone shot --crop 0,2000,1080,300  # an explicit X,Y,W,H rectangle
  phone shot --scale 0.3 --jpeg 60   # the whole screen for a fraction of the bytes
  phone shot --settle                # let the screen stop changing first

A full frame costs roughly 1500 tokens to read, and most looks are aimed at one
control. Crop it, or scale it down, or use `snapshot` instead when the answer is
text.

`--crop <name>` finds the element carrying that name, which for a card is its
label rather than the card. `--expand 1` widens to the box around it, `--expand
2` to the box around that; use it when what you want to see is the control and
its state rather than the words on it."#)]
    Shot {
        #[arg(id = "device")]
        target: Option<String>,

        /// Write to FILE instead of the clipboard; "-" writes the image to stdout
        #[arg(short, long)]
        out: Option<String>,

        /// Keep only part of the frame: an element by text, @index or X,Y,W,H
        #[arg(long)]
        crop: Option<String>,

        /// Crop to the Nth box around --crop instead: its row, card or dialog
        #[arg(long, requires = "crop", value_parser = clap::value_parser!(u8).range(1..=8))]
        expand: Option<u8>,

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
    #[command(after_help = r#"Examples:
  phone size  # on Android -> 1080x2400 pixels
  phone size  # on a simulator -> 402x874 points (1206x2622 pixels, scale 3)
  phone size --json

Android works in pixels throughout. A simulator reports bounds and takes taps in
points while screenshotting at 2x or 3x, so a coordinate measured by eye off an
iOS screenshot has to be divided by `scale` before it can be tapped."#)]
    Size {
        #[arg(id = "device")]
        target: Option<String>,

        /// Emit JSON instead of a line
        #[arg(long)]
        json: bool,
    },
    /// Press an element, named by its text, description, @index or X,Y
    #[command(after_help = r#"Examples:
  phone tap "Log in"  # by text, content description or resource id
  phone tap @62       # by the index `snapshot` printed
  phone tap 540,1200  # by coordinate, where there are no elements to name

An ambiguous name is refused with the candidates listed as @index rather than
guessed at. Those indices number the rows of one dump, so name an element by its
text unless the index came from the command immediately before."#)]
    Tap { what: String },
    /// Hold an element down, named as `tap` names one
    #[command(after_help = r#"Examples:
  phone press Settings  # held for 800ms
  phone press @12 --hold 2s
  phone press 540,1200 --hold 1s

A hold is what opens a context menu, starts an icon rearrange or selects text. A
tap on the same element does something else entirely."#)]
    Press {
        what: String,

        /// How long to keep the touch down
        #[arg(long, default_value = "800ms", value_parser = parse_duration)]
        hold: Duration,
    },
    /// Drag between two points, or scroll the screen a direction
    #[command(after_help = r#"Examples:
  phone swipe up                     # scrolls a list down: the content follows the finger
  phone swipe down --amount 0.3      # across less of the panel
  phone swipe left --duration 800ms  # slow enough to drag rather than fling
  phone swipe 540,1800 540,700       # between two points
  phone swipe Photos Trash           # or between two elements

--amount is a fraction of the panel and belongs to a directional swipe only. A
directional swipe stays clear of the edges, because a drag begun at the very
edge is a system gesture and never reaches the app."#)]
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
    /// Type into whatever holds focus
    #[command(after_help = r#"Examples:
  phone tap "Search"
  phone type "rust lang"
  phone key enter

Goes to whatever holds focus, so tap the field first. Printable ASCII only, and
anything else is refused outright: the device drops what it cannot spell and
still reports success."#)]
    Type { text: String },
    /// Send a key, by keycode name (back, home, enter, tab…)
    #[command(after_help = r#"Examples:
  phone key enter
  phone key home
  phone key app_switch
  phone key back  # on Android this navigates, and can leave a form losing it

A simulator has no back button; when a key is refused the ones it does take are
listed."#)]
    Key { name: String },
    /// Block until an element appears, or stop waiting and fail
    #[command(after_help = r#"Examples:
  phone wait Dashboard
  phone wait --gone "Loading..." --timeout 10s

Exits 0 the moment the answer is yes and non-zero if it never is, which is both
quicker and more honest than a sleep: a screenshot taken straight after a tap
returns the frame that was already up. Takes a name, not an @index."#)]
    Wait {
        what: String,

        /// Wait for it to leave the screen instead
        #[arg(long)]
        gone: bool,

        #[arg(long, default_value = "15s", value_parser = parse_duration)]
        timeout: Duration,
    },
    /// Run several screen verbs against one device, surveying once
    #[command(after_help = r#"Examples:
  phone do "tap 'Log in'" "wait Inbox" "shot --crop @2"
  phone do -t pixel_7-api36 "swipe up --amount 0.5" "wait Calendar" "snapshot"

Every invocation of `phone` surveys the hosts before it acts, which costs around
eight seconds whatever the verb is. `do` pays that once and runs each step
against the same device, so three steps cost eight seconds rather than
twenty-four.

Each step is a whole command, quoted, and takes the flags it takes on its own.
Steps run in order and stop at the first failure, which is reported with its
number. Only verbs that read or press a screen can be sequenced, and -t/--focus
belong on `do` rather than on a step."#)]
    Do {
        /// Each a whole command, quoted: `phone do "tap Login" "wait Inbox"`
        #[arg(required = true)]
        steps: Vec<String>,
    },

    /// What exists, how to reach it, and whether it is up
    #[command(after_help = r#"Examples:
  phone device                    # same as `phone device list`
  phone device list --json
  phone device use pixel_7-api36
  phone device boot medium_phone

A bare `phone device` lists. Everything a device is or has — its transport, its
pairing, whether it is running — is under here."#)]
    Device {
        #[command(subcommand)]
        action: Option<DeviceAction>,
    },
    /// Put an app on a device, start it, point it somewhere, watch it
    #[command(after_help = r#"Examples:
  phone app launch com.example.app
  phone app logs com.example.app

In a project with a phone.toml, `phone up` installs and starts the app already.
These are for the times that is not what is wanted: a one-off apk, a url, a log
stream."#)]
    App {
        #[command(subcommand)]
        action: AppAction,
    },
    /// Choose which ssh hosts to survey for devices
    #[command(after_help = r#"Examples:
  phone host              # what each host was found to offer
  phone host enable rose  # probe it, and survey it from then on
  phone host disable rose

Only enabled hosts are surveyed, so each one adds to what every command costs.
Enabling a host again re-probes what it can drive."#)]
    Host {
        #[command(subcommand)]
        action: Option<HostAction>,
    },

    // this machine, and the two verbs that want a person watching
    /// Watch the screen live in a window on this machine
    #[command(after_help = r#"Examples:
  phone mirror

Opens a scrcpy window on this machine, so it needs a display and is for a person
watching rather than for a script. To see a screen without one, use `snapshot`
or `shot`."#)]
    Mirror {
        #[arg(id = "device")]
        target: Option<String>,
    },
    /// Record the screen, and pull stills out of the clip
    #[command(after_help = r#"Examples:
  phone record                       # 5s, clip path printed
  phone record --seconds 12
  phone record --frames 4            # the clip plus 4 evenly spaced stills
  phone record --frames changed      # a still wherever the picture moved
  phone record --frames 6 --scale 0.4 --jpeg 60
  phone record --frames 3 -o /tmp/login.mp4

A screenshot says what is on screen; it cannot say what happened on the way
there. An animation that lands wrong, a splash that never clears, a swipe that
scrolled the inner list — those look the same before and after.

The clip itself is unreadable to anything that reads text, so `--frames N` cuts
it into N stills spanning the whole clip, first frame to last. They take
`--scale` and `--jpeg` the way `shot` does, and every path is printed.

Even spacing is the wrong sampling for a route change, which is short and
bunched: a tap whose animation is over in half a second gets one still of the
old screen and three of the settled new one. `--frames changed` picks the
moments the picture actually moved instead, still anchored by the first and
last frame, and says how big the gaps between them are.

A still that cannot be taken is reported and skipped; the clip and the rest of
the stills are still written, and the run only fails if none of them landed.

Without `-o` the clip lands under the state directory and its path is printed.
Android and simulators; up to 180 seconds."#)]
    Record {
        #[arg(id = "device", value_parser = not_a_length)]
        target: Option<String>,

        /// How long to record for
        #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=180))]
        seconds: u32,

        /// Also write stills: N evenly spaced, or `changed` for the moments the picture moved
        #[arg(long, value_parser = parse_frames)]
        frames: Option<crate::record::Frames>,

        /// Write the clip here instead of under the state directory
        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Resize the stills, 1.0 being the panel's own resolution
        #[arg(long, requires = "frames", value_parser = parse_scale)]
        scale: Option<f64>,

        /// Encode the stills as JPEG at this quality rather than PNG
        #[arg(long, requires = "frames", value_parser = clap::value_parser!(u8).range(1..=100))]
        jpeg: Option<u8>,
    },
    /// Check the tools and daemons this depends on
    #[command(after_help = r#"Examples:
  phone doctor

Reports what is missing rather than what is wrong with a device: adb, the
clipboard, the ssh hosts that answer and what each one still offers. Run it when
a command fails in a way that looks like a tool is absent, not when a device
will not respond."#)]
    Doctor,
    /// Print a shell completion script. Hidden: `default.nix` calls it in
    /// postInstall to generate the completion files, and nothing else does.
    #[command(hide = true)]
    Completions { shell: Shell },
}

#[derive(Subcommand)]
pub enum DeviceAction {
    /// List every reachable and remembered device
    #[command(after_help = r#"Examples:
  phone device list
  phone device list --json

The state column says what to do next: `attached` and `online` are ready to
drive, `off` exists but is not running and takes `boot`, `known` and `offline`
take `connect`."#)]
    List {
        /// Emit JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Bring up a transport to a device, trying history before discovery
    #[command(after_help = r#"Examples:
  phone device connect             # the most recent device
  phone device connect faraday
  phone device connect --no-sweep  # skip the port sweep when an address is remembered

Brings up a transport to a device that is already running. To start one that is
not, see `phone device boot`."#)]
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
    #[command(after_help = r#"Examples:
  phone device disconnect        # the device in use
  phone device disconnect --all  # every wireless transport at once

Only drops the transport. The device keeps running, and `connect` brings it back
without a new pairing."#)]
    Disconnect {
        #[arg(id = "device")]
        target: Option<String>,

        /// Drop every wireless transport
        #[arg(long)]
        all: bool,
    },
    /// Pair with a device that is in wireless-debugging pairing mode
    #[command(after_help = r#"Examples:
  phone device pair 314159
  phone device pair 314159 --addr 192.168.1.40:37021

Needed once per handset, from Developer options > Wireless debugging > Pair
device with pairing code. The code and the port are both shown in that dialog
and both expire when it closes. Emulators and simulators never need this."#)]
    Pair {
        /// Six digit code from the pairing dialog
        code: String,

        /// host:port from the dialog; discovered over mDNS when omitted
        #[arg(long)]
        addr: Option<String>,
    },
    /// Restart adbd on a fixed port so later reconnects skip discovery
    #[command(after_help = r#"Examples:
  phone device pin              # port 5555 on the device in use
  phone device pin --port 5678

A paired handset picks a fresh port every time it reconnects, which is what
makes `connect` sweep for it. Pinning one costs a few seconds now and saves that
sweep on every later connect. Does not survive a reboot of the device."#)]
    Pin {
        #[arg(id = "device")]
        target: Option<String>,

        #[arg(long, default_value_t = 5555)]
        port: u16,
    },
    /// Set the device other commands target by default
    #[command(after_help = r#"Examples:
  phone device use pixel_7-api36  # every later command targets it
  phone device use                # settle on the default already in force

Set once at the start of a session rather than passing -t to every command. A
-t or PHONE_TARGET on a single command still wins over it, and this wins over
the `default` a project's phone.toml names."#)]
    Use {
        #[arg(id = "device")]
        target: Option<String>,
    },
    /// Drop a device from the registry
    #[command(after_help = r#"Examples:
  phone device forget galaxy-s26-plus

Removes what was remembered about it: its address, its alias, its pairing. It
reappears the next time it is discovered, so this is for a device that is gone
for good or one whose remembered address is wrong."#)]
    Forget {
        #[arg(id = "device")]
        target: String,
    },
    /// Start a simulator or emulator and wait until it can be driven
    #[command(after_help = r#"Examples:
  phone device boot "iPhone 17 Pro"  # a simulator, on whichever host has it
  phone device boot medium_phone     # an AVD, here or on a host
  phone device boot --timeout 5m nyx-remote-android

Returns only once the device can actually be driven, not when the process
starts: a simulator answers immediately and an emulator shows its window well
before its system is up, and a tap sent at either moment is lost. Booting one
that is already running says so and exits 0, so it is safe in front of a script.

`phone device list` lists what can be booted as `off`. Expect 20-30s. This
starts something already defined; creating an AVD or a simulator is still
`avdmanager` or `simctl create`."#)]
    Boot {
        #[arg(id = "device")]
        target: Option<String>,

        /// Give up if it is still not usable by then
        #[arg(long, default_value = "180s", value_parser = parse_duration)]
        timeout: Duration,
    },
    /// Stop a running simulator or emulator
    #[command(after_help = r#"Examples:
  phone device shutdown medium_phone
  phone device shutdown "iPhone 17 Pro"

Stopping one that is not running is a no-op. Handsets cannot be stopped."#)]
    Shutdown {
        #[arg(id = "device")]
        target: Option<String>,
    },
    /// Point a port on the device at the same port on its host
    #[command(after_help = r#"Examples:
  phone device reverse 8081
  phone device reverse 8081:3000
  phone device reverse --list
  phone device reverse --clear

A dev server is reachable from a device only if the device has a port that
answers to it, and `localhost:8081` inside an emulator is the emulator, not the
machine the bundler is on. This is the forward that fixes that.

The port it reaches is on the machine running the adb server that holds the
device: for an emulator on a mac, this sends the device to that mac's loopback,
where a bundler started over ssh there is listening. One on this machine is not
what it will find.

`DEVICE:HOST` when the two differ. Android only — a simulator is already on its
host's loopback. A project's phone.toml declares these per device, and `phone
up` opens them."#)]
    Reverse {
        /// PORT, or DEVICE:HOST when the numbers differ
        #[arg(value_parser = parse_ports, conflicts_with_all = ["list", "clear"])]
        ports: Option<(u16, u16)>,

        /// Show the forwards this device already has
        #[arg(long, conflicts_with = "clear")]
        list: bool,

        /// Remove every forward on this device
        #[arg(long)]
        clear: bool,
    },
}

#[derive(Subcommand)]
pub enum AppAction {
    /// Install an apk, or a .app bundle on a simulator
    #[command(after_help = r#"Examples:
  phone app install ./app/build/outputs/apk/debug/app-debug.apk
  phone app install -t "iPhone 17 Pro" ./build/Debug-iphonesimulator/App.app

An .apk goes to a handset or an emulator, a .app bundle to a simulator. The file
is read here and sent to whichever host holds the device, so a path on this
machine is what it wants. `phone app launch <app>` starts what was installed."#)]
    Install { apk: PathBuf },
    /// Start an app, whatever is on screen
    #[command(after_help = r#"Examples:
  phone app launch com.example.app
  phone app launch -t "iPhone 17 Pro Max" com.example.app

Sends the intent the launcher icon sends, so the app comes up the way a person
starting it would find it. Returns once the process exists, which is what makes
the next `snapshot` a snapshot of the app rather than of whatever was in front.

A package name on Android, a bundle id on a simulator. The app has to be
installed already — `phone app install` puts it there."#)]
    Launch { app: String },
    /// Force-stop an app, leaving the device up
    #[command(after_help = r#"Examples:
  phone app stop com.example.app

This is the app, not the device: `phone device shutdown` is the one that turns a
device off. Stopping an app that was not running is not an error, and says so."#)]
    Stop { app: String },
    /// Open a url or a deep link on the device
    #[command(after_help = r#"Examples:
  phone app open https://example.com
  phone app open "exp+myapp://expo-development-client/?url=http://localhost:8081"

The device resolves the url, so a deep link lands in whichever app registered
the scheme and an http url lands in the browser. Quote it: a shell reads `?` and
`&` before this ever sees them."#)]
    Open { url: String },
    /// Stream logs for a package name or bundle id
    #[command(after_help = r#"Examples:
  phone app logs com.example.app
  phone app logs -t faraday com.example.app

A package name on Android, a bundle id on a simulator or an iPhone."#)]
    Logs { app: String },
}

#[derive(Subcommand)]
pub enum HostAction {
    /// Show every ssh host and what it was found to offer
    #[command(after_help = r#"Examples:
  phone host list"#)]
    List,

    /// Survey this host's devices from now on, probing what it can drive
    #[command(after_help = r#"Examples:
  phone host enable rose

Probes the host once for adb, the Android sdk and the iOS tools, remembers what
answered, and surveys it from then on."#)]
    Enable { name: String },

    /// Stop surveying it
    #[command(after_help = r#"Examples:
  phone host disable rose

Its devices stop appearing and every command gets quicker. What was remembered
about it is kept, so enabling it again does not re-probe from nothing."#)]
    Disable { name: String },
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

/// `--frames 4` is four evenly spaced stills, `--frames changed` however many
/// moments the picture moved in. One flag rather than two, because both answer
/// the same question and a caller should not have to know which one it is
/// asking before it can ask it.
fn parse_frames(s: &str) -> Result<crate::record::Frames, String> {
    use crate::record::Frames;

    if s.eq_ignore_ascii_case("changed") {
        return Ok(Frames::Changed);
    }

    match s.parse::<u8>() {
        Ok(n) if (1..=32).contains(&n) => Ok(Frames::Count(n)),
        Ok(_) => Err("between 1 and 32 stills, or `changed`".to_string()),
        Err(_) => Err(format!("{s} is neither a number of stills nor `changed`")),
    }
}

/// Every verb takes the device first, so `record 12` is a device named `12` —
/// survey and an ambiguity to say what `--seconds` says plainly. No device is
/// named for a number alone, so reading it as one costs nothing.
fn not_a_length(s: &str) -> Result<String, String> {
    if s.parse::<u32>().is_ok() {
        return Err(format!(
            "the device comes first here; for a length say --seconds {s}"
        ));
    }

    Ok(s.to_string())
}

/// `8081`, or `8081:3000` when the device port and the host port differ. Port 0
/// is refused rather than passed on: adb reads it as "pick one for me", and a
/// forward on a port nobody was told about is a forward nobody can use.
fn parse_ports(s: &str) -> Result<(u16, u16), String> {
    let (device, host) = s.split_once(':').unwrap_or((s, s));

    let port = |raw: &str, side| {
        raw.trim()
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .ok_or_else(|| match raw.trim() {
                // the two flags are the only other things this argument is
                // ever handed, and adb spells them the same way
                word @ ("list" | "clear") => format!("{word} is a flag here: --{word}"),
                raw => format!("{raw} is not a {side} port"),
            })
    };

    Ok((port(device, "device")?, port(host, "host")?))
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
    use clap::CommandFactory;

    #[test]
    fn the_word_and_the_count_are_the_same_flag() {
        use crate::record::Frames;

        assert_eq!(parse_frames("4"), Ok(Frames::Count(4)));
        assert_eq!(parse_frames("changed"), Ok(Frames::Changed));
        assert_eq!(parse_frames("Changed"), Ok(Frames::Changed));
        assert!(parse_frames("0").is_err());
        assert!(parse_frames("every").is_err());
    }

    #[test]
    fn a_bare_number_is_read_as_a_length_asked_for_in_the_wrong_place() {
        let err = not_a_length("12").unwrap_err();

        assert!(err.contains("--seconds 12"), "{err}");
        assert_eq!(
            not_a_length("pixel_7-api36"),
            Ok("pixel_7-api36".to_string())
        );
    }

    #[test]
    fn a_port_pair_reads_the_device_side_first() {
        assert_eq!(parse_ports("8081"), Ok((8081, 8081)));
        assert_eq!(parse_ports("8081:3000"), Ok((8081, 3000)));
        assert!(parse_ports("0").is_err());
        assert!(parse_ports("8081:0").is_err());
    }

    /// `adb reverse --list` is a flag and this reads like a subcommand, so the
    /// wrong one gets typed; the error is the only place to say which it is.
    #[test]
    fn the_flags_are_named_when_they_arrive_as_an_argument() {
        assert_eq!(
            parse_ports("list"),
            Err("list is a flag here: --list".to_string())
        );
        assert_eq!(
            parse_ports("clear"),
            Err("clear is a flag here: --clear".to_string())
        );
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

    /// The overview's table of commands, as `(heading, verbs)`. A heading that
    /// is itself a command names a group and its verbs are that group's; one
    /// that is not — `project`, `screen`, `this` — is only a way of reading
    /// the list, and its verbs are top level.
    fn overview_rows() -> Vec<(&'static str, Vec<&'static str>)> {
        OVERVIEW
            .lines()
            .skip_while(|line| *line != "The commands")
            .take_while(|line| !line.starts_with("What to know"))
            .filter_map(|line| line.strip_prefix("  "))
            .filter(|line| !line.starts_with(' ') && !line.is_empty())
            .filter_map(|line| line.split_once("  "))
            .map(|(head, verbs)| (head.trim(), verbs.split_whitespace().collect()))
            .collect()
    }

    fn subcommands(cmd: &clap::Command) -> Vec<String> {
        cmd.get_subcommands()
            .filter(|sub| !sub.is_hide_set())
            .map(|sub| sub.get_name().to_string())
            .filter(|name| name != "help")
            .collect()
    }

    /// The overview groups the verbs and `--help` lists them; a reader who
    /// learns the order in one has to find it in the other. Nothing in clap
    /// enforces that, so it is enforced here.
    #[test]
    fn the_overview_lists_every_verb_in_the_order_help_does() {
        let cli = Cli::command();
        let top = subcommands(&cli);

        let mut expected: Vec<String> = Vec::new();

        for (head, verbs) in overview_rows() {
            // a heading that is a command of its own is the group, and what
            // follows it belongs to that group rather than to the top level
            match top.iter().any(|name| name == head) {
                true => expected.push(head.to_string()),
                false => expected.extend(verbs.iter().map(|verb| verb.to_string())),
            }
        }

        assert_eq!(expected, top, "the overview and the command list disagree");
    }

    /// A group is only worth grouping if the overview says what is in it, and
    /// only useful if that list is the real one.
    #[test]
    fn every_group_lists_the_verbs_it_actually_has() {
        let cli = Cli::command();
        let top = subcommands(&cli);

        let mut checked = 0;

        for (head, verbs) in overview_rows() {
            if !top.iter().any(|name| name == head) {
                continue;
            }

            let group = cli
                .get_subcommands()
                .find(|sub| sub.get_name() == head)
                .expect("named in the top level list");

            assert_eq!(
                verbs,
                subcommands(group),
                "the overview and `phone {head} --help` disagree"
            );

            checked += 1;
        }

        assert_eq!(checked, 3, "device, app and host are the groups");
    }

    /// Every invocation printed under `--help`, from the overview and from each
    /// verb alike. A line is a command up to its `#`, which is what lets the
    /// examples be checked rather than merely written.
    fn documented_invocations() -> Vec<String> {
        fn collect(cmd: &clap::Command, out: &mut Vec<String>) {
            if let Some(help) = cmd.get_after_help() {
                for line in help.to_string().lines() {
                    let line = line.trim();

                    if let Some(rest) = line.strip_prefix("phone ") {
                        let command = rest.split(" #").next().unwrap_or(rest);

                        out.push(command.trim().to_string());
                    }
                }
            }

            for sub in cmd.get_subcommands() {
                collect(sub, out);
            }
        }

        let mut out = Vec::new();
        collect(&Cli::command(), &mut out);

        out
    }

    /// Help that does not parse is worse than no help: an agent reads it, runs
    /// it verbatim and gets a usage error back from the tool that suggested it.
    #[test]
    fn every_example_in_the_help_runs() {
        let examples = documented_invocations();

        assert!(
            examples.len() > 65,
            "expected the help to carry examples, found {}",
            examples.len()
        );

        for example in examples {
            let words = shell_words::split(&example)
                .unwrap_or_else(|e| panic!("`phone {example}` is not a command line: {e}"));

            if let Err(e) = Cli::try_parse_from(std::iter::once("phone".to_string()).chain(words)) {
                panic!(
                    "`phone {example}` is in --help but does not parse:\n{}",
                    e.to_string().lines().next().unwrap_or_default()
                );
            }
        }
    }

    /// A subcommand that names an argument the same as a global one silently
    /// takes it over, and the global stops reaching that subcommand at all.
    #[test]
    fn a_subcommand_does_not_shadow_a_global_flag() {
        Cli::command().debug_assert();

        let cli = Cli::try_parse_from(["phone", "size", "-t", "faraday"]).unwrap();
        assert_eq!(cli.target.as_deref(), Some("faraday"));
    }

    /// `-t` has to survive being written after a group as well as after a verb,
    /// which is the position it is typed in for everything under `app`.
    #[test]
    fn a_global_flag_reaches_through_a_group_to_its_verb() {
        let cli =
            Cli::try_parse_from(["phone", "app", "launch", "-t", "faraday", "com.example.app"])
                .unwrap();

        assert_eq!(cli.target.as_deref(), Some("faraday"));
    }

    /// A group with nothing after it is a question about what it holds, and the
    /// two whose answer is a list say it rather than a usage error.
    #[test]
    fn the_groups_that_list_can_be_asked_bare() {
        let bare = |args: &[&str]| {
            Cli::try_parse_from(std::iter::once("phone").chain(args.iter().copied())).is_ok()
        };

        assert!(bare(&["device"]));
        assert!(bare(&["host"]));
        assert!(!bare(&["app"]), "there is nothing to list without a device");
    }
}
