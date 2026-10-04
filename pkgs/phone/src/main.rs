#![forbid(unsafe_code)]

// Shadows std's printing macros crate-wide, so whatever a run says lands in its
// call log too; textual scope reaches every module declared below.
macro_rules! print {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        std::print!("{text}");
        crate::calls::out(&text);
    }};
}

macro_rules! println {
    () => { print!("\n") };
    ($($arg:tt)*) => { print!("{}\n", format_args!($($arg)*)) };
}

macro_rules! eprint {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        std::eprint!("{text}");
        crate::calls::err(&text);
    }};
}

macro_rules! eprintln {
    () => { eprint!("\n") };
    ($($arg:tt)*) => { eprint!("{}\n", format_args!($($arg)*)) };
}

mod a11y;
mod actions;
mod adb;
mod answer;
mod apps;
mod avd;
mod calls;
mod cli;
mod connect;
mod discover;
mod doctor;
mod grid;
mod help;
mod hook;
mod hosts;
mod ios;
mod lease;
mod memory;
mod model;
mod picker;
mod pids;
mod project;
mod record;
mod registry;
mod scroll;
mod simctl;
mod ssh;
mod stamps;
mod stream;
mod tui;
mod up;
mod usage;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::time::Duration;

use std::os::unix::process::CommandExt;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser};
use tokio::sync::mpsc::UnboundedReceiver;

use actions::Sink;
use adb::Server;
use cli::{AppAction, Cli, Command, DeviceAction, HostAction, DEFAULT_AMOUNT};
use connect::{Reporter, Step};
use discover::survey;
use model::{Platform, View};
use project::Project;
use registry::Registry;

#[tokio::main]
async fn main() -> ExitCode {
    // Rust ignores SIGPIPE, so a write to a closed pipe comes back as an error
    // that `println!` panics on. `phone snapshot | head` is how a long dump is
    // read, and it must end the run quietly rather than in a backtrace.
    sigpipe::reset();
    calls::begin();

    let cli = help::parse();

    match &cli.command {
        Some(Command::Hook { .. } | Command::Completions { .. }) => calls::skip(),
        None | Some(Command::Stream { .. } | Command::Mirror { .. } | Command::Record { .. }) => {
            calls::quiet()
        }
        Some(_) => {}
    }

    if let Some(Command::Hook { harness }) = cli.command {
        hook::run(harness);

        return ExitCode::SUCCESS;
    }

    let code = match dispatch(cli).await {
        Ok(()) => 0,
        Err(e) => {
            // the alternate form walks the context chain; without it a failure
            // reads as "in phone.toml" with the reason it failed dropped
            eprintln!("phone: {e:#}");

            match e.downcast_ref::<Refused>() {
                Some(_) => 3,
                None => 1,
            }
        }
    };

    calls::finish(Some(i32::from(code)), None);

    ExitCode::from(code)
}

#[derive(Debug)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

fn exit_on_drift(report: &up::Report, ours: Option<&str>) {
    use std::io::Write;

    let code = report.exit(ours);

    if code == 0 {
        return;
    }

    if let Some(ours) = ours.filter(|_| code == 4) {
        eprintln!("phone: {ours} is in place; only other declared devices drifted (`status -t {ours}` checks it alone)");
    }

    std::io::stdout().flush().ok();
    calls::finish(Some(code), None);
    std::process::exit(code);
}

async fn dispatch(cli: Cli) -> Result<()> {
    if let Some(Command::Completions { shell }) = cli.command {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();

        clap_complete::generate(
            clap_complete::Shell::from(shell),
            &mut cmd,
            name,
            &mut std::io::stdout(),
        );

        return Ok(());
    }

    let mut reg = Registry::load()?;

    let Cli {
        command,
        target: fallback,
        focus,
    } = cli;

    // the positional form wins: it was typed at this command, rather than
    // inherited from a `PHONE_TARGET` left in the environment
    let want = |positional: Option<String>| {
        positional
            .or_else(|| fallback.clone())
            .filter(|s| !s.is_empty())
    };

    // a verb that reads or presses a screen needs a device, and finding one costs
    // a survey of every enabled host. They are gathered here so that `do` can pay
    // for it once and hand the same device to each step.
    if let Some(positional) = command.as_ref().and_then(Command::on_screen) {
        let want = want(positional.map(str::to_string));
        let hold = Hold::for_verb(command.as_ref(), focus);
        let session = Session::open(&mut reg, want.as_deref(), focus, hold).await?;

        return answered(&session, command.expect("classified as a screen verb")).await;
    }

    match command {
        None => {
            if let Some(mut cmd) = tui::run(reg).await? {
                calls::exec(&cmd);
                let err = cmd.exec();

                bail!("{err}");
            }

            Ok(())
        }

        Some(Command::Up {
            profile,
            rebuild,
            take,
            over_budget,
            timeout,
        }) => {
            let project = declared()?;

            let opts = up::Opts {
                profile,
                rebuild,
                take,
                over_budget,
                timeout,
            };

            up::up(&mut reg, &project, &opts).await
        }

        Some(Command::Down) => {
            let project = declared()?;

            up::down(&mut reg, &project).await
        }

        Some(Command::Status { profile, json }) => {
            let project = declared()?;
            let report = up::status(
                &mut reg,
                &project,
                profile.as_deref(),
                want(None).as_deref(),
            )
            .await?;

            match json {
                true => println!("{}", serde_json::to_string_pretty(&report)?),
                false => up::print(&report),
            }

            let ours = want(None)
                .or_else(|| std::env::var("PHONE_TARGET").ok())
                .filter(|s| !s.is_empty())
                .or(project.manifest.default);

            exit_on_drift(&report, ours.as_deref());

            Ok(())
        }

        // a bare `phone device` is the question the list answers
        Some(Command::Device { action }) => {
            match action.unwrap_or(DeviceAction::List { json: false }) {
                DeviceAction::List { json } => list(&mut reg, json).await,
                DeviceAction::Connect {
                    target,
                    no_sweep,
                    range,
                    concurrency,
                } => {
                    let view =
                        resolve(&mut reg, want(target).as_deref(), true, Aim::Running).await?;

                    let opts = connect::Opts {
                        sweep: !no_sweep,
                        range: range.unwrap_or(discover::sweep::EPHEMERAL),
                        concurrency,
                    };

                    let (rep, drain) = reporter();
                    let serial =
                        connect::connect(&mut reg, &view.server, &view.device, &opts, &rep).await;

                    drop(rep);
                    drain.await;

                    println!("{}", serial?);

                    Ok(())
                }
                DeviceAction::Disconnect { target, all } => {
                    disconnect(&mut reg, want(target), all).await
                }
                DeviceAction::Pair { code, addr } => {
                    let (rep, drain) = reporter();
                    let res = connect::pair(&Server::Local, addr.as_deref(), &code, &rep).await;

                    drop(rep);
                    drain.await;

                    res?;

                    eprintln!("phone: now run `phone device connect` to bring the transport up");

                    Ok(())
                }
                DeviceAction::Pin { target, port } => {
                    let view =
                        resolve(&mut reg, want(target).as_deref(), true, Aim::Running).await?;

                    let (rep, drain) = reporter();
                    let res = connect::pin(&mut reg, &view.server, &view.device, port, &rep).await;

                    drop(rep);
                    drain.await;

                    res
                }
                DeviceAction::Use { target } => {
                    let view = resolve(&mut reg, target.as_deref(), false, Aim::Running).await?;

                    reg.current = Some(view.device.id.clone());
                    reg.save()?;

                    eprintln!("phone: default target is {}", view.device.label);

                    Ok(())
                }
                DeviceAction::Forget { target } => forget(&mut reg, &target),
                DeviceAction::Boot {
                    target,
                    over_budget,
                    timeout,
                } => boot_device(&mut reg, want(target), over_budget, timeout).await,
                DeviceAction::Clone { target, new } => clone_device(&mut reg, &target, &new).await,
                DeviceAction::Delete { target, yes } => delete_device(&mut reg, &target, yes).await,
                DeviceAction::Shutdown { target } => {
                    let view =
                        resolve(&mut reg, want(target).as_deref(), true, Aim::Running).await?;

                    // turning off the device somebody else is holding ends their session
                    lease::check(&view).await?;

                    eprintln!("phone: {}", actions::stop(&view.device, &view.reach).await?);

                    Ok(())
                }
                DeviceAction::Net { state, only } => net(&mut reg, want(None), state, only).await,
                DeviceAction::Reverse { ports, list, clear } => {
                    let view = driving(&mut reg, want(None).as_deref(), true).await?;

                    let what = match (ports, list, clear) {
                        (Some((device, host)), ..) => actions::Reverse::Open { device, host },
                        (None, _, true) => actions::Reverse::Clear,
                        // a bare `phone device reverse` is a question, not a broken command:
                        // nothing was named to forward, so say what is forwarded
                        (None, ..) => actions::Reverse::List,
                    };

                    let said = actions::reverse(&view.server, &view.device, what).await?;

                    // a listing is the one form worth piping into something; the rest
                    // is the same status line every other verb writes
                    match list || ports.is_none() && !clear {
                        true => println!("{said}"),
                        false => eprintln!("phone: {said}"),
                    }

                    Ok(())
                }
            }
        }

        Some(Command::App { action }) => apps_cmd(&mut reg, want(None), action).await,

        Some(Command::Host { action }) => hosts_cmd(&mut reg, action).await,

        Some(Command::Stream {
            target,
            size,
            base64,
            shm,
        }) => {
            stream_cmd(
                &mut reg,
                want(target),
                size,
                stream::Output::new(shm, base64),
            )
            .await
        }

        Some(Command::Mirror { target }) => {
            let view = driving(&mut reg, want(target).as_deref(), true).await?;

            eprintln!(
                "phone: {}",
                actions::mirror(&view.server, &view.device).await?
            );

            Ok(())
        }

        Some(Command::Record {
            target,
            seconds,
            frames,
            out,
            scale,
            jpeg,
        }) => {
            let view = driving(&mut reg, want(target).as_deref(), true).await?;

            let take = record::Take {
                seconds,
                frames,
                out,
                shot: actions::Shot {
                    scale,
                    jpeg,
                    ..Default::default()
                },
            };

            let (rep, drain) = reporter();
            let res = record::record(&view.server, &view.device, &take, &rep).await;

            drop(rep);
            drain.await;

            for path in res? {
                println!("{}", path.display());
            }

            Ok(())
        }

        Some(Command::Doctor) => doctor::run(&mut reg).await,

        Some(Command::Completions { .. } | Command::Hook { .. }) => unreachable!("handled above"),

        // every screen verb returned above, where it was given a device
        Some(_) => unreachable!("a screen verb reached dispatch"),
    }
}

/// Lists or toggles the ssh hosts a survey reaches into. The names come from
/// ssh, and how to reach one is already answered by the user's `ssh_config`.
async fn disconnect(reg: &mut Registry, want: Option<String>, all: bool) -> Result<()> {
    if all {
        adb::run(&Server::Local, &["disconnect"]).await?;
        eprintln!("phone: dropped every wireless transport");

        return Ok(());
    }

    let view = resolve(reg, want.as_deref(), true, Aim::Running).await?;

    let Some(serial) = view.reach.serial().filter(|s| s.contains(':')) else {
        bail!("{} has no wireless transport", view.device.label);
    };

    adb::disconnect(&view.server, serial).await?;
    eprintln!("phone: disconnected {serial}");

    Ok(())
}

fn forget(reg: &mut Registry, target: &str) -> Result<()> {
    let matches = reg.find(target);

    let Some(id) = matches.first().map(|d| d.id.clone()) else {
        bail!("nothing in the registry matches '{target}'");
    };

    if matches.len() > 1 {
        bail!(
            "'{target}' matches {} devices; be more specific",
            matches.len()
        );
    }

    reg.remove(&id);
    reg.save()?;

    eprintln!("phone: forgot {id}");

    Ok(())
}

async fn clone_device(reg: &mut Registry, target: &str, new: &str) -> Result<()> {
    let views = survey(reg).await;
    reg.save()?;

    let view = choose(&views, reg, Some(target), false, Aim::Bootable).await?;

    if views.iter().any(|v| {
        v.device.host == view.device.host
            && v.device.label.eq_ignore_ascii_case(new)
    }) {
        bail!(
            "{} already has a device named {new}",
            actions::where_of(&view.device).label()
        );
    }

    eprintln!(
        "phone: {}",
        actions::clone(&view.device, &view.reach, new).await?
    );

    Ok(())
}

async fn delete_device(reg: &mut Registry, target: &str, yes: bool) -> Result<()> {
    let views = survey(reg).await;
    reg.save()?;

    let view = choose(&views, reg, Some(target), false, Aim::Bootable).await?;
    let said = actions::delete(&view.device, &view.reach, yes).await?;

    reg.remove(&view.device.id);
    reg.save()?;

    eprintln!("phone: {said}");

    Ok(())
}

async fn boot_device(
    reg: &mut Registry,
    want: Option<String>,
    over_budget: bool,
    timeout: std::time::Duration,
) -> Result<()> {
    let views = survey(reg).await;
    reg.save()?;

    let view = choose(&views, reg, want.as_deref(), true, Aim::Bootable).await?;

    if !actions::running(&view.reach) {
        memory::admit(reg, &views, &[&view.device], over_budget).await?;
    }

    boot(reg, view, timeout).await
}

async fn net(
    reg: &mut Registry,
    want: Option<String>,
    state: cli::Switch,
    only: Option<cli::Radio>,
) -> Result<()> {
    let view = driving(reg, want.as_deref(), true).await?;
    let on = matches!(state, cli::Switch::On);

    eprintln!(
        "phone: {}",
        actions::net(&view.server, &view.device, on, only).await?
    );

    Ok(())
}

async fn apps_cmd(reg: &mut Registry, want: Option<String>, action: AppAction) -> Result<()> {
    let view = driving(reg, want.as_deref(), true).await?;
    let (server, device) = (&view.server, &view.device);

    let said = match action {
        AppAction::Install { apk } => {
            let (rep, drain) = reporter();
            let res = actions::install(server, device, &apk, &rep).await;

            drop(rep);
            drain.await;

            res?
        }
        AppAction::Launch { app, extras } => {
            apps::launch(server, device, &app, &[], &extras).await?
        }
        AppAction::Stop { app } => apps::stop(server, device, &app).await?,
        AppAction::Open { url } => {
            eprintln!("phone: {}", apps::open(server, device, &url).await?);

            return Ok(());
        }
        AppAction::Notifications { app } => {
            println!(
                "{}",
                apps::notifications(server, device, app.as_deref()).await?
            );

            return Ok(());
        }
        AppAction::Logs { app } => {
            let mut cmd = actions::logs_command(server, device, &app).await?;

            calls::exec(&cmd);

            bail!("{}", cmd.exec())
        }
    };

    eprintln!("phone: {said}");

    pids::forget(&device.id);

    Ok(())
}

async fn stream_cmd(
    reg: &mut Registry,
    want: Option<String>,
    size: stream::Size,
    output: stream::Output,
) -> Result<()> {
    let view = reaching(reg, want.as_deref(), true, Hold::Ignored).await?;

    stream::stream(&view.server, &view.device, size, output).await
}

async fn hosts_cmd(reg: &mut Registry, action: Option<HostAction>) -> Result<()> {
    let found = hosts::discover().await;
    let names: Vec<String> = found.iter().map(|h| h.name.clone()).collect();

    reg.sync_hosts(&names);

    match action {
        Some(HostAction::Enable { name }) => {
            // whether ssh has a stanza for the name is not this program's business:
            // MagicDNS, /etc/hosts and plain DNS all reach real machines. Probing on
            // enable, not per survey — an ssh round trip is not a per-refresh cost.
            let Some(caps) = hosts::probe(&name).await else {
                bail!("{name} did not answer; `ssh {name} true` says why");
            };

            let state = reg.host_mut(&name);

            state.enabled = true;
            hosts::stamp(state, caps);

            reg.save()?;

            eprintln!("phone: {name} enabled ({})", caps.label());

            if !caps.any() {
                eprintln!("phone: nothing to drive there; adb, xcrun and tunneld all answered no");
            }

            Ok(())
        }

        Some(HostAction::Budget {
            name,
            reserve,
            emulator_overhead,
            simulator,
        }) => {
            let state = reg.host_mut(&name);
            let mut budget = state.budget.unwrap_or_default();

            for (set, to) in [
                (&mut budget.reserve, reserve),
                (&mut budget.emulator_overhead, emulator_overhead),
                (&mut budget.simulator, simulator),
            ] {
                if let Some(to) = to {
                    if !(to >= 0.0 && to.is_finite()) {
                        bail!("{to} is not a size in GB");
                    }

                    *set = to;
                }
            }

            if reserve.or(emulator_overhead).or(simulator).is_some() {
                state.budget = Some(budget);
                reg.save()?;
            }

            println!("{name}: {}", budget.label());

            Ok(())
        }

        Some(HostAction::Disable { name }) => {
            reg.host_mut(&name).enabled = false;
            reg.save()?;

            eprintln!("phone: {name} disabled");

            Ok(())
        }

        None | Some(HostAction::List) => {
            if reg.hosts.is_empty() {
                eprintln!("phone: no Host stanzas in your ssh config");

                return Ok(());
            }

            for state in &reg.hosts {
                let detail = if state.enabled {
                    match state.tunnel_port {
                        Some(port) => format!("{} · adb on :{port}", state.caps.label()),
                        None => state.caps.label(),
                    }
                } else {
                    found
                        .iter()
                        .find(|h| h.name == state.name)
                        .map(|h| h.target.clone())
                        .unwrap_or_default()
                };

                println!(
                    "  {} {:<20} {detail}",
                    if state.enabled { "◉" } else { "○" },
                    state.name
                );
            }

            reg.save()?;

            Ok(())
        }
    }
}

async fn list(reg: &mut Registry, json: bool) -> Result<()> {
    let mut views = survey(reg).await;
    reg.save()?;

    let hosts = memory::hosts_of(&views);
    let (ledgers, mine, rooms) = tokio::join!(lease::ledgers(&views), lease::mine(), async {
        match json {
            true => Vec::new(),
            false => memory::rooms(reg, &views, &hosts).await,
        }
    });
    let holds = lease::holds(&views, &ledgers);
    let driven = usage::driven(&views, &ledgers, mine.as_ref().map(|m| m.tree.as_str()));

    usage::rank(&mut views, |v| {
        driven.get(&v.device.id).copied().unwrap_or_default()
    });

    if json {
        return print_json(&views, &holds, &driven);
    }

    for (at, room) in &rooms {
        println!(
            "{}",
            usage::summary(at, &views, &driven, room.as_ref().ok())
        );
    }

    print_table(&views, &holds, &driven, mine.as_ref());

    for line in memory::lines(&rooms, memory::Room::advice) {
        println!("{line}");
    }

    Ok(())
}

/// The manifest the project verbs act on. Not finding one is the mistake that
/// actually gets made — the command was typed outside the checkout — so the
/// error says what was looked for and where, not that a file is missing.
fn declared() -> Result<Project> {
    let here = std::env::current_dir()?;

    Project::here()?.ok_or_else(|| {
        anyhow::anyhow!(
            "no {} in {} or any directory above it",
            project::FILE,
            here.display()
        )
    })
}

/// The device a project would rather have, for when nothing typed names one. A
/// manifest that does not parse reads as no preference at all: `phone up` is
/// where that is reported, and a broken file has no business taking the screen
/// verbs down with it.
fn preferred() -> Option<String> {
    Project::here().ok().flatten()?.manifest.default
}

/// A reporter whose steps stream to stderr, so stdout stays reserved for the
/// one thing a caller might pipe (`shot -o -`, the serial from `connect`).
fn reporter() -> (Reporter, impl std::future::Future<Output = ()>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    (Reporter::new(tx), drain(rx))
}

async fn drain(mut rx: UnboundedReceiver<Step>) {
    let mut last_percent = 0;

    while let Some(step) = rx.recv().await {
        match step {
            Step::Try(t) => eprintln!("  · {t}"),
            Step::Done(t) => eprintln!("  ✓ {t}"),
            Step::Fail(t) => eprintln!("  ✗ {t}"),
            Step::Note(t) => eprintln!("  • {t}"),
            Step::Progress { done, total } => {
                let percent = done * 100 / total.max(1);

                if percent >= last_percent + 10 {
                    last_percent = percent;
                    eprintln!("    {percent}%");
                }
            }
        }
    }
}

/// One device, resolved once. Every screen verb needs the same two things — the
/// host and transport that reach it, and the accessibility target layered on top
/// — and finding them costs a survey of every enabled ssh host.
struct Session {
    view: View,
    target: a11y::Target,
    focus: Option<(i32, i32)>,
    before: RefCell<Option<Vec<u8>>>,
    read: RefCell<Option<a11y::Screen>>,
    acted: Cell<bool>,
}

impl Session {
    async fn open(
        reg: &mut Registry,
        want: Option<&str>,
        focus: Option<(i32, i32)>,
        hold: Hold,
    ) -> Result<Self> {
        let view = reaching(reg, want, true, hold).await?;
        let target = target_of(&view, focus).await?;

        Ok(Session {
            view,
            target,
            focus,
            before: RefCell::new(None),
            read: RefCell::new(None),
            acted: Cell::new(false),
        })
    }

    fn pick<'a>(&self, screen: &'a a11y::Screen, what: &str) -> Result<&'a a11y::Node> {
        let shown = a11y::recall(&self.view.device.id);

        a11y::pick_in(&screen.nodes, what, shown.as_deref()).inspect_err(|e| {
            if e.is::<a11y::Ambiguous>() {
                self.remember(&screen.nodes);
            }
        })
    }

    fn remember(&self, nodes: &[a11y::Node]) {
        if let Err(e) = a11y::remember(&self.view.device.id, nodes) {
            eprintln!("phone: could not keep these rows for @index: {e:#}");
        }
    }
}

/// A screen verb against an already-resolved device.
async fn step(s: &Session, command: Command) -> Result<()> {
    match command {
        Command::Shot {
            target,
            out,
            crop,
            expand,
            pad,
            scale,
            jpeg,
            settle,
            grid,
        } => {
            let _ = target;

            // a frame is the whole display whatever holds focus, so --focus only
            // reaches the dump that resolves --crop
            if s.focus.is_some() && crop.is_none() {
                bail!("--focus picks the window a dump reads; a frame has no window");
            }

            // reading the frame and reading the elements in it are two calls to
            // the same device, so the crop is worked out off this one view
            let crop = crop_of(s, crop.as_deref(), expand, pad).await?;

            let sink = Sink::from_opt(out.as_deref());
            let shot = actions::Shot {
                crop,
                scale,
                jpeg,
                settle,
                grid: grid_scale(s, grid).await?,
            };

            let before = s.before.take();

            let (rep, drain) = reporter();
            let res = actions::screenshot_after(
                &s.view.server,
                &s.view.device,
                &sink,
                &rep,
                &shot,
                before,
            )
            .await;

            drop(rep);
            drain.await;

            eprintln!("phone: {}", res?);

            Ok(())
        }

        Command::Size { target, json } => {
            let _ = target;
            let t = &s.target;
            let size = a11y::size(t).await?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "width": size.width,
                        "height": size.height,
                        "scale": size.scale,
                        "pixels": [size.width * size.scale, size.height * size.scale],
                    }))?
                );

                return Ok(());
            }

            // the second half is worth printing only where the two spaces differ
            if size.scale == 1.0 {
                println!("{}x{} pixels", size.width, size.height);
            } else {
                println!(
                    "{}x{} points ({}x{} pixels, scale {})",
                    size.width,
                    size.height,
                    size.width * size.scale,
                    size.height * size.scale,
                    size.scale,
                );
            }

            Ok(())
        }

        Command::Snapshot { target, json } => {
            // uiautomator has no display flag; it reads whichever one holds focus
            let _ = target;
            let screen = a11y::dump(&s.target).await?;

            s.remember(&screen.nodes);
            *s.read.borrow_mut() = Some(screen.clone());

            if json {
                print_elements_json(&screen)?;
            } else {
                print_elements(&screen);
            }

            Ok(())
        }

        Command::Tap {
            what,
            double,
            fingers,
            force,
        } => {
            eprintln!("phone: {}", tap(s, &what, double, fingers.into(), force).await?);

            Ok(())
        }

        Command::Press { what, hold, force } => {
            let t = &s.target;
            let ((x, y), name) = at(s, &what, force).await?;

            // a device tells a press from a tap by how long the touch lasts, not
            // by where it went, so a hold is a drag that stays where it started
            a11y::swipe(t, (x, y), (x, y), hold).await?;
            eprintln!(
                "phone: held {} for {}ms",
                aim((x, y), name),
                hold.as_millis()
            );

            Ok(())
        }

        Command::Swipe { edge: Some(edge), amount, duration, .. } => scroll::edge(&s.target, edge, amount, duration).await,
        command @ Command::Swipe { until: Some(_), .. } => scroll::until(s, command).await,
        Command::Swipe {
            from: Some(from),
            to,
            duration,
            hold,
            amount,
            fingers,
            force,
            ..
        } => {
            let (from, to) = ends(s, &from, to.as_deref(), amount, force).await?;

            eprintln!("phone: {}", swipe(s, (from, to), duration, hold, fingers.into()).await?);

            Ok(())
        }

        Command::Pinch {
            what,
            factor,
            duration,
            angle,
            force,
        } => {
            eprintln!("phone: {}", pinch(s, &what, factor, duration, angle, force).await?);

            Ok(())
        }

        Command::Rotate {
            what,
            degrees,
            duration,
            force,
        } => {
            eprintln!("phone: {}", rotate(s, &what, degrees, duration, force).await?);

            Ok(())
        }

        Command::Wait {
            what,
            gone,
            timeout,
        } => {
            if what.starts_with('@') {
                bail!("@index numbers one dump and `wait` takes many; name the element");
            }

            let t = &s.target;

            let prior = s.read.take();

            wait(
                t,
                &what,
                gone,
                timeout,
                prior.as_ref().map(|p| p.nodes.as_slice()),
            )
            .await
        }

        Command::Type { text } => {
            let t = &s.target;

            calls::typed(&text);

            a11y::type_text(t, &text).await?;
            eprintln!("phone: typed {} characters", text.chars().count());

            Ok(())
        }

        Command::Fill { what, text, force } => {
            eprintln!("phone: {}", fill(s, &what, &text, force).await?);

            Ok(())
        }

        Command::Key { name } => {
            eprintln!("phone: {}", a11y::key(&s.target, &name).await?);

            Ok(())
        }
        Command::Do { steps } => sequence(s, &steps).await,

        // `on_screen` is what routed this here, so nothing else can arrive
        _ => unreachable!("not a screen verb"),
    }
}

/// Runs each step against the one device, stopping at the first that fails. The
/// steps are whole commands rather than bare arguments so that every flag keeps
/// the meaning it has on its own, and so that a caller can build one from the
/// same strings it would have typed.
async fn sequence(s: &Session, steps: &[String]) -> Result<()> {
    let commands = steps
        .iter()
        .enumerate()
        .map(|(n, raw)| parse_step(n, raw))
        .collect::<Result<Vec<_>>>()?;

    let settles: Vec<bool> = commands
        .iter()
        .map(|c| matches!(c, Command::Shot { settle: true, .. }))
        .collect();

    for (n, command) in commands.into_iter().enumerate() {
        let raw = &steps[n];

        // named before it runs, not after: a step that hangs is the one an agent
        // needs to see, and its own line only arrives once it is over
        eprintln!("  [{}/{}] {raw}", n + 1, steps.len());

        if command.acts() && settles.get(n + 1) == Some(&true) {
            let (rep, drain) = reporter();
            let frame = actions::capture(&s.view.server, &s.view.device, &rep).await;

            drop(rep);
            drain.await;

            *s.before.borrow_mut() = frame.ok();
        }

        let acts = command.acts();
        let last = n + 1 == steps.len();

        Box::pin(async move {
            match last {
                true => answered(s, command).await,
                false => step(s, command).await,
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("step {} ({raw}): {e}", n + 1))?;

        s.acted.set(s.acted.get() || acts);
    }

    Ok(())
}

async fn answered(s: &Session, command: Command) -> Result<()> {
    if !command.acts() {
        return step(s, command).await;
    }

    s.read.take();

    let first = reads_first(&command);
    let (early, frame) = tokio::join!(
        async {
            match first {
                true => None,
                false => a11y::dump(&s.target).await.ok(),
            }
        },
        frame(s),
    );

    step(s, command).await?;

    let Some(before) = early.or_else(|| s.read.take()) else {
        return Ok(());
    };

    match answer::after(&s.target, &before).await {
        Ok(change) if change.how == answer::How::Unmoved => {
            println!("{}", unmoved(s, frame, &change.screen).await)
        }
        Ok(change) => print_change(s, &before, &change),
        Err(e) => eprintln!(
            "phone: the act went through, but the screen after it could not be read: {e:#}"
        ),
    }

    Ok(())
}

fn reads_first(command: &Command) -> bool {
    let named = |what: &str| cli::parse_point(what).is_err();

    match command {
        Command::Tap { what, .. }
        | Command::Press { what, .. }
        | Command::Pinch { what, .. }
        | Command::Rotate { what, .. } => named(what),
        Command::Swipe {
            from: Some(from),
            to: Some(to),
            ..
        } => named(from) || named(to),
        Command::Fill { .. } => true,
        _ => false,
    }
}

const APPEARED_SHOWN: usize = 15;
const GONE_SHOWN: usize = 3;

async fn frame(s: &Session) -> Option<Vec<u8>> {
    let (rep, drain) = reporter();
    let png = actions::capture(&s.view.server, &s.view.device, &rep).await;

    drop(rep);
    drain.await;

    png.ok()
}

fn decode(png: &[u8]) -> Option<image::RgbImage> {
    Some(image::load_from_memory(png).ok()?.to_rgb8())
}

async fn unmoved(s: &Session, before: Option<Vec<u8>>, screen: &a11y::Screen) -> String {
    let drawn = match before {
        Some(before) => match (decode(&before), frame(s).await.as_deref().and_then(decode)) {
            (Some(before), Some(after)) => Some(answer::drawn(&before, &after)),
            _ => None,
        },
        None => None,
    };

    let scale = match (&drawn, &s.target) {
        (Some(answer::Drawn::At(_)), a11y::Target::Simulator(_)) => {
            a11y::size(&s.target).await.ok().map(|z| z.scale)
        }
        _ => Some(1.0),
    };

    match (drawn, scale) {
        (None, _) => format!(
            "unchanged  no row changed within {}s; a slow result still shows up with `wait`, and `shot --grid` shows what the act hit",
            actions::MOVE_LIMIT.as_secs()
        ),
        (Some(answer::Drawn::Still), _) => {
            "unchanged  tree and pixels both still; the act most likely hit nothing that reacts, `shot --grid` shows where it landed".to_string()
        }
        (Some(answer::Drawn::At(px)), Some(scale)) => {
            let at = answer::in_points(px, scale);
            let rows = a11y::rows(&screen.nodes);
            let inside = answer::holder(&rows, at)
                .map(|r| {
                    let index = a11y::recall(&s.view.device.id)
                        .and_then(|shown| shown.iter().position(|v| *v == r.node.signature()))
                        .map(|i| format!(" @{i}"))
                        .unwrap_or_default();

                    format!(" inside {}{index}", a11y::row_label(&r.label))
                })
                .unwrap_or_default();

            format!(
                "unchanged  tree still, but pixels changed at {},{}-{},{}{inside}; drawn content the tree cannot read: look with `shot --crop {},{},{},{}`",
                at.x1,
                at.y1,
                at.x2,
                at.y2,
                at.x1,
                at.y1,
                at.width(),
                at.height()
            )
        }
        (Some(_), _) => {
            "unchanged  tree still, but pixels changed; drawn content the tree cannot read: look with `shot`".to_string()
        }
    }
}

fn print_change(s: &Session, before: &a11y::Screen, change: &answer::Change) {
    let screen = &change.screen;

    let still = match change.how {
        answer::How::Restless => {
            format!(", still changing at {}s", actions::SETTLE_LIMIT.as_secs())
        }
        _ => String::new(),
    };

    println!(
        "changed    {} new, {} gone{still}",
        change.appeared.len(),
        change.gone.len()
    );

    if let Some(keyboard) = screen.keyboard.filter(|k| Some(*k) != before.keyboard) {
        println!("keyboard   {}", keyboard.describe());
    }

    let all = a11y::rows(&screen.nodes);
    let rows: Vec<&a11y::Row> = change
        .appeared
        .iter()
        .take(APPEARED_SHOWN)
        .map(|&i| &all[i])
        .collect();
    let nodes: Vec<&a11y::Node> = rows.iter().map(|r| r.node).collect();

    let mut shown = a11y::recall(&s.view.device.id).unwrap_or_default();

    for (row, index) in rows.iter().zip(answer::number(&mut shown, &nodes)) {
        print_row(screen, row, index);
    }

    if let Err(e) = a11y::remember_rows(&s.view.device.id, &shown) {
        eprintln!("phone: could not keep these rows for @index: {e:#}");
    }

    let more = change.appeared.len().saturating_sub(APPEARED_SHOWN);

    if more > 0 {
        println!("…          {more} more new; `phone snapshot` lists them");
    }

    let rest = match change.gone.len().saturating_sub(GONE_SHOWN) {
        0 => String::new(),
        n => format!(" (+{n} more)"),
    };

    if !change.gone.is_empty() {
        let names: Vec<String> = change
            .gone
            .iter()
            .take(GONE_SHOWN)
            .map(|l| a11y::row_label(l))
            .collect();

        println!("gone       {}{rest}", names.join(" | "));
    }
}

fn parse_step(n: usize, raw: &str) -> Result<Command> {
    let words = shell_words::split(raw).map_err(|e| anyhow::anyhow!("step {}: {e}", n + 1))?;

    let parsed =
        Cli::try_parse_from(std::iter::once("phone".to_string()).chain(words)).map_err(|e| {
            anyhow::anyhow!(
                "step {} ({raw}): {}{}",
                n + 1,
                first_line(&e.to_string()),
                help::misplaced(&e)
                    .map(|t| format!("; {t}"))
                    .unwrap_or_default()
            )
        })?;

    // the device and the window were settled before the first step ran, and a
    // step that names either would be describing a different session
    if parsed.target.is_some() || parsed.focus.is_some() {
        bail!(
            "step {} ({raw}): --target and --focus belong on `do`, not on a step",
            n + 1
        );
    }

    let Some(command) = parsed.command else {
        bail!("step {} ({raw}): no verb", n + 1);
    };

    if command.on_screen().is_none() {
        bail!(
            "step {} ({raw}): only verbs that read or press a screen can be sequenced",
            n + 1
        );
    }

    if matches!(command, Command::Do { .. }) {
        bail!("step {} ({raw}): a sequence does not nest", n + 1);
    }

    Ok(command)
}

/// clap renders a usage block under its message, which reads as noise once the
/// message is already being quoted inside a step's own error.
fn first_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or(text)
        .trim_start_matches("error: ")
        .to_string()
}

/// A device to read and press. `uiautomator` and `input` ride the adb transport,
/// so a handset here and an emulator on a mac behave alike. A simulator has no
/// transport at all — CoreSimulator is macOS-local — so its verbs run on the
/// host, which needs a `phone` of its own. An iPhone has neither.
async fn target_of(view: &View, focus: Option<(i32, i32)>) -> Result<a11y::Target> {
    if view.device.platform == Platform::Simulator {
        if focus.is_some() {
            bail!("--focus picks between displays; a simulator has one");
        }

        return Ok(a11y::Target::Simulator(a11y::Simulator {
            at: actions::where_of(&view.device),
            udid: simctl::udid(&view.device)?.to_string(),
        }));
    }

    if view.device.platform.is_hosted() {
        bail!(
            "cannot read or press {} — {} has no adb transport",
            view.device.label,
            view.device.platform
        );
    }

    let serial = connect::serial_of(&view.server, &view.device).await?;

    Ok(a11y::Target::Adb(a11y::Adb {
        display: adb::active_display(&view.server, &serial).await,
        server: view.server.clone(),
        serial,
        focus,
        device: Box::new(view.device.clone()),
    }))
}

/// A point to aim at, and what it turned out to be. `X,Y` is taken literally —
/// a canvas, a map, an unfocused split half — and anything else names an
/// element. The name comes back so that a tap, a hold and a drag all report the
/// same way; a caller that resolved an element wants to see which one.
async fn at(s: &Session, what: &str, force: bool) -> Result<((i32, i32), Option<String>)> {
    if let Ok(point) = cli::parse_point(what) {
        return Ok((point, None));
    }

    let (screen, node) = find(s, what).await?;

    if !force {
        refuse_covered(&screen, &node)?;
    }

    Ok((pressable(&screen, &node)?, Some(node.label())))
}

fn pressable(screen: &a11y::Screen, node: &a11y::Node) -> Result<(i32, i32)> {
    screen.reach(node).point().ok_or_else(|| {
        let b = node.bounds;

        anyhow::anyhow!(
            "{} is off the panel at [{},{}][{},{}]; swipe it into view first",
            node.label(),
            b.x1,
            b.y1,
            b.x2,
            b.y2
        )
    })
}

async fn ends(
    s: &Session,
    from: &str,
    to: Option<&str>,
    amount: f64,
    force: bool,
) -> Result<((i32, i32), (i32, i32))> {
    let Some(to) = to else {
        let direction = from
            .parse()
            .map_err(|e| anyhow::anyhow!("{e}; a swipe from a point needs somewhere to go"))?;

        return Ok(a11y::along(a11y::size(&s.target).await?, direction, amount));
    };

    if amount != DEFAULT_AMOUNT {
        bail!("--amount sizes a directional swipe; this one has both ends");
    }

    Ok((at(s, from, force).await?.0, at(s, to, force).await?.0))
}

const APPEAR_GRACE: Duration = Duration::from_secs(2);

/// Right after an act the next screen may still be drawing or sliding in, and a
/// touch mid-transition is dropped: a name is read until it is there and still.
async fn find(s: &Session, what: &str) -> Result<(a11y::Screen, a11y::Node)> {
    let began = std::time::Instant::now();
    let mut seen = None;

    loop {
        let screen = a11y::dump(&s.target).await?;
        *s.read.borrow_mut() = Some(screen.clone());
        let late = began.elapsed() >= APPEAR_GRACE;

        match s.pick(&screen, what).cloned() {
            Ok(node) if !s.acted.get() || seen == Some(node.bounds) || late => {
                return Ok((screen, node));
            }
            Ok(node) => seen = Some(node.bounds),
            Err(e) if e.is::<a11y::Missing>() && !late => {}
            Err(e) => return Err(e),
        }
    }
}

fn refuse_covered(screen: &a11y::Screen, node: &a11y::Node) -> Result<()> {
    if screen.covered(node) {
        let (x, y) = pressable(screen, node)?;

        bail!(
            "{} at {x},{y} is under the keyboard, which would take the touch; \
             `phone key hide_keyboard` first, or --force",
            node.label()
        );
    }

    Ok(())
}

const FIELD_READS: usize = 4;

fn related(a: &a11y::Node, b: &a11y::Node) -> bool {
    a.bounds.holds(b.bounds.center()) || b.bounds.holds(a.bounds.center())
}

fn editable(node: &a11y::Node) -> bool {
    node.class.contains("EditText") || node.class.contains("TextField") || !node.hint.is_empty()
}

struct Aimed {
    centre: (i32, i32),
    within: a11y::Bounds,
    size: a11y::Size,
    name: Option<String>,
}

async fn aimed(s: &Session, what: &str, force: bool) -> Result<Aimed> {
    let size = a11y::size(&s.target).await?;
    let panel = a11y::Bounds {
        x1: 0,
        y1: 0,
        x2: size.width as i32,
        y2: size.height as i32,
    };

    let (centre, within, name) = match cli::parse_point(what) {
        Ok(point) => (point, panel, None),
        Err(_) => {
            let (screen, node) = find(s, what).await?;

            if !force {
                refuse_covered(&screen, &node)?;
            }

            let within = node.bounds.clipped(&panel).unwrap_or(node.bounds);

            (pressable(&screen, &node)?, within, Some(node.label()))
        }
    };

    Ok(Aimed {
        centre,
        within,
        size,
        name,
    })
}

async fn pinch(s: &Session, what: &str, factor: f64, duration: Duration, angle: f64, force: bool) -> Result<String> {
    let on = aimed(s, what, force).await?;
    let fingers = a11y::fingers(on.centre, on.within, on.size, factor, angle)?;
    a11y::gesture(&s.target, &fingers, duration).await?;

    let (from, to) = fingers.gap();
    let way = if factor > 1.0 { "out" } else { "in" };

    Ok(format!(
        "pinched {way} on {}, fingers {from} to {to} apart over {}ms",
        aim(on.centre, on.name),
        duration.as_millis()
    ))
}

async fn rotate(s: &Session, what: &str, degrees: f64, duration: Duration, force: bool) -> Result<String> {
    let on = aimed(s, what, force).await?;
    let fingers = a11y::twist(on.centre, on.within, on.size, degrees)?;
    a11y::gesture(&s.target, &fingers, duration).await?;

    let way = if degrees > 0.0 { "clockwise" } else { "anticlockwise" };

    Ok(format!(
        "turned two fingers {}° {way} on {}, {} apart, over {}ms",
        degrees.abs(),
        aim(on.centre, on.name),
        fingers.gap().0,
        duration.as_millis()
    ))
}

fn counted(fingers: usize) -> String {
    match fingers {
        1 => String::new(),
        n => format!(" with {n} fingers"),
    }
}

async fn tap(s: &Session, what: &str, double: bool, fingers: usize, force: bool) -> Result<String> {
    let t = &s.target;
    let ((x, y), name) = at(s, what, force).await?;

    if !double && fingers == 1 {
        a11y::tap(t, x, y).await?;

        return Ok(format!("tapped {}", aim((x, y), name)));
    }

    let taps = if double { 2 } else { 1 };
    let g = a11y::Gesture {
        taps,
        ..a11y::side_by_side((x, y), (x, y), a11y::size(t).await?, fingers)
    };
    a11y::gesture(t, &g, a11y::TAP_DOWN).await?;

    let how = if double { "double-tapped" } else { "tapped" };

    Ok(format!("{how} {}{}", aim((x, y), name), counted(fingers)))
}

async fn swipe(
    s: &Session,
    (from, to): ((i32, i32), (i32, i32)),
    duration: Duration,
    hold: Option<Duration>,
    fingers: usize,
) -> Result<String> {
    let t = &s.target;

    match (hold, fingers) {
        (Some(_), 2..) => bail!("--hold drags one finger; drop --hold or --fingers"),
        (Some(hold), _) => a11y::drag(t, from, to, hold, duration).await?,
        (None, 1) => a11y::swipe(t, from, to, duration).await?,
        (None, _) => {
            let g = a11y::side_by_side(from, to, a11y::size(t).await?, fingers);
            a11y::gesture(t, &g, duration).await?;
        }
    }

    let held = hold
        .map(|h| format!(" after holding {}ms", h.as_millis()))
        .unwrap_or_default();

    Ok(format!(
        "swiped {},{} to {},{}{} over {}ms{held}",
        from.0,
        from.1,
        to.0,
        to.1,
        counted(fingers),
        duration.as_millis()
    ))
}

async fn fill(s: &Session, what: &str, text: &str, force: bool) -> Result<String> {
    let t = &s.target;

    if !matches!(t, a11y::Target::Adb(_)) {
        bail!("fill reads a field back through Android's dump; on a simulator tap and type");
    }

    a11y::sendable(text)?;

    let (screen, target) = find(s, what).await?;

    calls::secret_if(target.password, text);

    let already = screen.focused().filter(|f| related(f, &target)).cloned();

    let field = match already {
        Some(field) => field,
        None => tap_into(t, &screen, &target, force).await?,
    };

    if !field.is_empty_field() {
        a11y::clear(t, field.text.chars().count()).await?;
    }

    if !text.is_empty() {
        a11y::type_text(t, text).await?;
    }

    let label = field.field_name();

    if field.password {
        calls::secret(text);

        return Ok(format!(
            "filled {label} with {} characters (a password, not read back)",
            text.chars().count()
        ));
    }

    let mut last = None;

    for attempt in 0..FIELD_READS {
        if attempt > 0 {
            tokio::time::sleep(POLL).await;
        }

        let screen = a11y::dump(t).await?;
        let now = screen
            .nodes
            .iter()
            .find(|n| n.bounds == field.bounds && n.class == field.class)
            .or_else(|| screen.focused())
            .cloned();

        match now {
            Some(now) if now.reads(text) => {
                return Ok(match text.is_empty() {
                    true => format!("emptied {label}"),
                    false => format!("filled {label}, which reads {text:?}"),
                });
            }
            other => last = other,
        }
    }

    match last {
        Some(now) => {
            let actual = if now.is_empty_field() {
                ""
            } else {
                now.text.as_str()
            };

            bail!("{label} reads {actual:?} rather than {text:?}")
        }
        None => bail!("{label} lost focus while being filled; nothing on screen holds it"),
    }
}

async fn tap_into(
    t: &a11y::Target,
    screen: &a11y::Screen,
    target: &a11y::Node,
    force: bool,
) -> Result<a11y::Node> {
    if !force {
        refuse_covered(screen, target)?;
    }

    let (x, y) = pressable(screen, target)?;
    a11y::tap(t, x, y).await?;

    focus_after_tap(t, target).await
}

async fn focus_after_tap(t: &a11y::Target, target: &a11y::Node) -> Result<a11y::Node> {
    let mut stray = None;

    for _ in 0..FIELD_READS {
        tokio::time::sleep(POLL).await;

        let screen = a11y::dump(t).await?;

        match screen.focused() {
            Some(f) if related(f, target) || editable(f) => return Ok(f.clone()),
            Some(f) => stray = Some(f.label()),
            None => {}
        }
    }

    match stray {
        Some(other) => bail!("tapped {} but focus went to {other}", target.label()),
        None => bail!("tapped {} but nothing took focus", target.label()),
    }
}

/// `at 416,1627` for a bare point, `Gmail at 416,1627` for a named element.
fn aim(point: (i32, i32), name: Option<String>) -> String {
    match name {
        Some(name) => format!("{name} at {},{}", point.0, point.1),
        None => format!("{},{}", point.0, point.1),
    }
}

async fn grid_scale(s: &Session, grid: bool) -> Result<Option<f64>> {
    Ok(match grid {
        true => Some(a11y::size(&s.target).await?.scale),
        false => None,
    })
}

async fn crop_of(
    s: &Session,
    spec: Option<&str>,
    expand: Option<u8>,
    pad: i32,
) -> Result<Option<a11y::Bounds>> {
    Ok(match spec {
        Some(spec) => Some(crop_bounds(s, spec, expand, pad).await?),
        None => None,
    })
}

/// The part of the frame to keep, in pixels. An element is padded because a
/// crop tight to its bounds shows a control with nothing around it to say where
/// on the screen it is; an explicit rectangle is taken as given.
async fn crop_bounds(
    s: &Session,
    spec: &str,
    expand: Option<u8>,
    pad: i32,
) -> Result<a11y::Bounds> {
    let t = &s.target;
    let size = a11y::size(t).await?;
    let panel = (size.width as i32, size.height as i32);

    let bounds = match rect(spec) {
        Some(bounds) => bounds,
        // a rectangle short of four numbers is a typo, not the name of a
        // control, and looking for an element called "100,200" says nothing
        None if spec.split(',').all(|v| v.trim().parse::<i32>().is_ok()) => {
            bail!(
                "a crop rectangle is X,Y,W,H; '{spec}' has {} numbers",
                spec.split(',').count()
            )
        }
        None => {
            let screen = a11y::dump(t).await?;
            let node = s.pick(&screen, spec)?;

            let bounds = match expand {
                None => node.bounds,
                Some(levels) => widen(node, levels)?,
            };

            bounds.padded(pad, Some(panel))
        }
    };

    Ok(size.in_pixels(bounds))
}

/// The box `levels` up from `node`. The two ways this comes back empty are
/// worth telling apart: an element with no enclosing box at all is a screen
/// with nothing above it, while a snapshot that carries no boxes for anything
/// came from a receiver that predates the field.
fn widen(node: &a11y::Node, levels: u8) -> Result<a11y::Bounds> {
    if node.ancestors.is_empty() {
        bail!(
            "this device reports no boxes around '{}', so --expand has nothing to widen to \
             (a simulator needs a receiver new enough to send them)",
            node.label()
        );
    }

    node.enclosing(levels as usize).ok_or_else(|| {
        anyhow::anyhow!(
            "'{}' is not {levels} boxes deep; it sits inside {}",
            node.label(),
            match node.enclosures() {
                0 => "nothing bigger than itself".to_string(),
                n => format!("{n} of them"),
            }
        )
    })
}

/// `X,Y,W,H`, in the space element bounds are reported in.
fn rect(spec: &str) -> Option<a11y::Bounds> {
    let n: Vec<i32> = spec
        .split(',')
        .map(|v| v.trim().parse())
        .collect::<Result<_, _>>()
        .ok()?;

    let [x, y, w, h] = n[..] else {
        return None;
    };

    Some(a11y::Bounds {
        x1: x,
        y1: y,
        x2: x + w,
        y2: y + h,
    })
}

/// How often the screen is re-read while waiting. A dump costs a round trip and
/// a uiautomator pass, so this is a poll rather than anything finer.
const POLL: std::time::Duration = std::time::Duration::from_millis(500);

/// Blocks until an element is on screen, or gone. More honest than a fixed
/// sleep in both directions: it returns as soon as the screen is ready rather
/// than at the end of a guess, and it fails loudly when the screen never gets
/// there instead of acting on whatever was up at the time.
async fn wait(
    t: &a11y::Target,
    what: &str,
    gone: bool,
    timeout: std::time::Duration,
    prior: Option<&[a11y::Node]>,
) -> Result<()> {
    let started = std::time::Instant::now();
    let mut first = true;

    loop {
        // a dump that fails mid-transition is not an answer either way, but one
        // that never goes idle will not answer by the timeout either
        let screen = match a11y::dump(t).await {
            Ok(screen) => Some(screen),
            Err(e) if e.is::<a11y::NotIdle>() => return Err(e),
            Err(_) => None,
        };
        let present = screen
            .as_ref()
            .map_or(gone, |s| a11y::present(&s.nodes, what));

        if present != gone {
            let state = if gone { "gone" } else { "on screen" };

            match (first, prior.map(|p| a11y::present(p, what) != gone)) {
                (true, Some(true)) => {
                    eprintln!("phone: '{what}' was already {state} before the last step; nothing changed")
                }
                (true, None) => eprintln!(
                    "phone: '{what}' is {state} at the first look, so this wait cannot tell whether your last action did it"
                ),
                _ => eprintln!(
                    "phone: '{what}' {} after {:.1}s",
                    if gone { "left" } else { "appeared" },
                    started.elapsed().as_secs_f64()
                ),
            }

            return Ok(());
        }

        if started.elapsed() >= timeout {
            bail!(
                "'{what}' was still {} after {:.0}s{}",
                if gone { "there" } else { "missing" },
                timeout.as_secs_f64(),
                screen
                    .filter(|_| !gone)
                    .map_or_else(String::new, |s| instead(&s.nodes, what))
            );
        }

        tokio::time::sleep(POLL).await;
        first = false;
    }
}

fn instead(nodes: &[a11y::Node], what: &str) -> String {
    match a11y::near(nodes, what) {
        close if close.is_empty() => on_screen(
            &a11y::rows(nodes)
                .iter()
                .filter(|r| r.within.is_none())
                .map(|r| a11y::row_label(&r.label))
                .collect::<Vec<_>>(),
        ),
        close => close,
    }
}

const ON_SCREEN_SHOWN: usize = 6;

fn on_screen(labels: &[String]) -> String {
    let named: Vec<&str> = labels
        .iter()
        .map(String::as_str)
        .filter(|l| !l.is_empty())
        .collect();

    if named.is_empty() {
        return "; nothing on screen".to_string();
    }

    let rest = match named.len().saturating_sub(ON_SCREEN_SHOWN) {
        0 => String::new(),
        n => format!(" (+{n} more)"),
    };

    format!(
        "; on screen: {}{rest}",
        named[..named.len().min(ON_SCREEN_SHOWN)].join(" | ")
    )
}

fn print_elements(screen: &a11y::Screen) {
    if let Some(keyboard) = &screen.keyboard {
        let focus = screen
            .focused()
            .map(a11y::Node::describe_field)
            .unwrap_or_else(|| "nothing".to_string());

        println!("focus     {focus}");
        println!("keyboard  {}", keyboard.describe());
        println!();
    }

    for row in a11y::rows(&screen.nodes)
        .iter()
        .filter(|r| r.within.is_none())
    {
        print_row(screen, row, row.node.index);
    }
}

fn print_row(screen: &a11y::Screen, row: &a11y::Row, index: usize) {
    let node = row.node;
    let reach = screen.reach(node);
    let at = reach
        .point()
        .map_or_else(|| "-".to_string(), |(x, y)| format!("{x},{y}"));
    let press = match (node.clickable, reach) {
        (true, a11y::Reach::On(_) | a11y::Reach::Partly { .. }) => "tap",
        _ => "   ",
    };
    let note = reach.note().map(|n| format!("  {n}")).unwrap_or_default();
    let under = if screen.covered(node) {
        "  under keyboard"
    } else {
        ""
    };

    println!(
        "@{:<3} {press}  {:<40} {at}{note}{under}",
        index,
        a11y::row_label(&row.label)
    );
}

fn print_elements_json(screen: &a11y::Screen) -> Result<()> {
    let rows: Vec<serde_json::Value> = a11y::rows(&screen.nodes)
        .into_iter()
        .map(|row| {
            let node = row.node;
            let reach = screen.reach(node);

            serde_json::json!({
                "ref": format!("@{}", node.index),
                "label": row.label,
                "text": node.text,
                "desc": node.desc,
                "id": node.res_id,
                "clickable": node.clickable,
                "focused": node.focused,
                "covered": screen.covered(node),
                "at": reach.point().map(|(x, y)| [x, y]),
                "off_panel": reach.note(),
                "within": row.within.map(|p| format!("@{p}")),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}

async fn boot(reg: &mut Registry, view: View, timeout: Duration) -> Result<()> {
    if actions::running(&view.reach) {
        eprintln!("phone: {} is already running", view.device.label);

        return Ok(());
    }

    let (rep, drain) = reporter();
    let res = actions::boot(&view.device, timeout, &rep).await;

    drop(rep);
    drain.await;

    eprintln!("phone: {}", res?);

    let (live, notes) = actions::arrive(reg, &view).await?;

    for note in notes {
        eprintln!("phone: {note}");
    }

    println!("{} is {}", live.device.label, live.reach.label());

    Ok(())
}

/// Resolves a device that is about to be read, pressed or handed an app, which
/// is every verb except the ones whose job is to change what a device *is*:
/// `boot`, `connect`, `use`, `forget`.
///
/// A device that is defined and not running is the state a caller hits
/// constantly and fixes with one command, and every layer below this words it
/// differently — simctl says "Unable to lookup in current state: Shutdown", the
/// receiver says "not booted", `simctl io` says nothing at all and waits out the
/// timeout. All three answer with a udid rather than the name that was typed,
/// and none names the fix. The survey that finds the device already knows, so
/// it is answered here once, in the name it was asked in.
async fn driving(reg: &mut Registry, want: Option<&str>, prefer_recent: bool) -> Result<View> {
    reaching(reg, want, prefer_recent, Hold::Respected).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hold {
    Respected,
    Ignored,
}

impl Hold {
    fn for_verb(command: Option<&Command>, focus: Option<(i32, i32)>) -> Self {
        match focus.is_none() && command.is_some_and(Command::reads) {
            true => Hold::Ignored,
            false => Hold::Respected,
        }
    }
}

async fn reaching(
    reg: &mut Registry,
    want: Option<&str>,
    prefer_recent: bool,
    hold: Hold,
) -> Result<View> {
    let named = want
        .map(str::to_string)
        .or_else(|| std::env::var("PHONE_TARGET").ok())
        .filter(|s| !s.is_empty())
        .or_else(preferred);

    let views = match &named {
        Some(w) => {
            discover::survey_until(reg, |views| {
                views
                    .iter()
                    .any(|v| v.device.is(w) && actions::running(&v.reach))
            })
            .await
        }
        None => survey(reg).await,
    };

    reg.save()?;

    let view = choose(&views, reg, want, prefer_recent, Aim::Running).await?;

    if view.reach == model::Reach::Off {
        let fit = memory::fit(reg, &views, &view.device).await;

        bail!(off(&view.device.label, fit));
    }

    if let Some(why) = connect::stranded(&views, &view) {
        bail!(why);
    }

    let (held, looked, ()) = tokio::join!(
        async {
            match hold {
                Hold::Respected => lease::check(&view).await,
                Hold::Ignored => Ok(()),
            }
        },
        pids::look(&view),
        usage::stamp(&view.device)
    );

    held?;

    if let Some(looked) = looked {
        looked.settle();
    }

    Ok(view)
}

fn off(label: &str, fit: Option<memory::Fit>) -> String {
    let boot = format!("phone device boot {}", quoted(label));

    match fit {
        Some(memory::Fit::Room(room)) => format!("{label} is off; {room}: `{boot}`"),
        Some(memory::Fit::Full(why)) => format!(
            "{label} is off and there is no room to boot it now: {}\n\
             wait for a `phone down` or a release, or `{boot} --over-budget` boots it anyway; ask before using it",
            why.join("\n")
        ),
        None => format!("{label} is off; start it with `{boot}`"),
    }
}

/// A name with a space in it is one argument only if the shell is told so, and
/// every simulator is named like that.
fn quoted(label: &str) -> String {
    match label.contains(char::is_whitespace) {
        true => format!("\"{label}\""),
        false => label.to_string(),
    }
}

/// `prefer_recent` makes a bare `phone device connect` one keystroke: with nothing
/// to go on it reaches for the last device used rather than a mostly-offline picker.
async fn resolve(
    reg: &mut Registry,
    want: Option<&str>,
    prefer_recent: bool,
    aim: Aim,
) -> Result<View> {
    let views = survey(reg).await;
    reg.save()?;

    choose(&views, reg, want, prefer_recent, aim).await
}

async fn choose(
    views: &[View],
    reg: &Registry,
    want: Option<&str>,
    prefer_recent: bool,
    aim: Aim,
) -> Result<View> {
    let view = chosen(views, reg, want, prefer_recent, aim).await?;

    calls::device(&view.device);

    Ok(view)
}

async fn chosen(
    views: &[View],
    reg: &Registry,
    want: Option<&str>,
    prefer_recent: bool,
    aim: Aim,
) -> Result<View> {
    let want = want
        .map(str::to_string)
        .or_else(|| std::env::var("PHONE_TARGET").ok())
        .filter(|s| !s.is_empty());

    if let Some((host, kind)) = want.as_deref().and_then(|w| host_target(views, w)) {
        return on_host(views, reg, &host, kind, aim).await;
    }

    let mut candidates = candidates(views, want.as_deref(), aim);

    if candidates.is_empty() {
        bail!(match want {
            Some(w) => format!("no device matching '{w}'"),
            None => "no device reachable (check: adb devices, tailscale status)".to_string(),
        });
    }

    if let Some(view) = want
        .is_none()
        .then(|| untargeted(&candidates, reg, prefer_recent))
        .flatten()
    {
        return Ok(view);
    }

    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }

    let index = picker::pick(&candidates, "phone").await?;

    Ok(candidates.remove(index))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Aim {
    Running,
    Bootable,
}

fn untargeted(candidates: &[View], reg: &Registry, prefer_recent: bool) -> Option<View> {
    let current = reg
        .current
        .as_ref()
        .and_then(|id| candidates.iter().find(|v| v.device.id == *id));

    // a project names the device it prefers, and only a typed name overrules
    // it: what is remembered is one machine-wide choice every agent shares
    let named = || preferred().and_then(|name| candidates.iter().find(|v| v.device.is(&name)));

    let recent = || {
        candidates
            .iter()
            .filter(|_| prefer_recent)
            .filter(|v| v.device.last_connected.is_some())
            .max_by_key(|v| v.device.last_connected.unwrap_or(0))
    };

    named().or(current).or_else(recent).cloned()
}

fn candidates(views: &[View], want: Option<&str>, aim: Aim) -> Vec<View> {
    let Some(w) = want else {
        return views.to_vec();
    };

    let exact: Vec<View> = views
        .iter()
        .filter(|v| v.device.is(w) || v.answers_to(w))
        .cloned()
        .collect();

    if exact.is_empty() {
        return views
            .iter()
            .filter(|v| v.device.matches(w))
            .cloned()
            .collect();
    }

    let running = |v: &View| actions::running(&v.reach);
    let off = |v: &View| v.reach == model::Reach::Off;
    let named = |v: &View| v.device.is(w);

    let by: &[&dyn Fn(&View) -> bool] = match aim {
        Aim::Running => &[&running, &named],
        Aim::Bootable => &[&off, &running, &named],
    };

    let mut left = exact;

    for keep in by {
        if left.len() <= 1 {
            break;
        }

        let hits: Vec<View> = left.iter().filter(|v| keep(v)).cloned().collect();

        if !hits.is_empty() {
            left = hits;
        }
    }

    left
}

fn host_target(views: &[View], want: &str) -> Option<(String, Option<Platform>)> {
    if views
        .iter()
        .any(|v| v.device.is(want) || v.answers_to(want))
    {
        return None;
    }

    let (host, kind) = match want.split_once('/') {
        Some((host, kind)) => {
            let kind = [Platform::Emulator, Platform::Simulator]
                .into_iter()
                .find(|p| p.as_str().eq_ignore_ascii_case(kind))?;

            (host, Some(kind))
        }
        None => (want, None),
    };

    let host = views
        .iter()
        .filter_map(|v| v.device.host.as_deref())
        .find(|h| h.eq_ignore_ascii_case(host))?;

    Some((host.to_string(), kind))
}

fn pool<'a>(views: &'a [View], host: &str, kind: Option<Platform>) -> Vec<&'a View> {
    views
        .iter()
        .filter(|v| matches!(v.device.platform, Platform::Emulator | Platform::Simulator))
        .filter(|v| kind.is_none_or(|k| v.device.platform == k))
        .filter(|v| v.device.host.as_deref() == Some(host))
        .collect()
}

fn pick<'a>(
    pool: &[&'a View],
    driven: &BTreeMap<String, usage::Driven>,
    free: impl Fn(&View) -> bool,
) -> Option<&'a View> {
    let mut up: Vec<&View> = pool
        .iter()
        .copied()
        .filter(|v| actions::running(&v.reach) && free(v))
        .collect();

    usage::rank(&mut up, |v| {
        driven.get(&v.device.id).copied().unwrap_or_default()
    });

    up.first().copied()
}

fn stranded(
    host: &str,
    pool: &[&View],
    driven: &BTreeMap<String, usage::Driven>,
    room: Option<&memory::Room>,
) -> String {
    let mut said = vec![match pool.iter().any(|v| actions::running(&v.reach)) {
        true => format!("everything up on {host} is held by another project"),
        false => format!("nothing on {host} is up"),
    }];

    let off: Vec<&View> = pool
        .iter()
        .copied()
        .filter(|v| v.reach == model::Reach::Off)
        .collect();
    let usual = usage::usual(&off, driven);

    match usual {
        Some((view, true)) => {
            said.push(format!("this project usually boots {}", view.device.label))
        }
        Some((view, false)) => said.push(format!("{} is the most driven there", view.device.label)),
        None => {}
    }

    let fit = room
        .zip(usual)
        .and_then(|(room, (view, _))| room.room_for(view.device.platform));

    if let Some(room) = room {
        said.push(match fit {
            Some(n) if n > 0 => format!("room to boot {n}"),
            _ => room.brief(),
        });
    }

    said.push(match (usual, fit) {
        (_, Some(0)) => format!("`phone device list` shows who holds what on {host}"),
        (Some((view, _)), _) => format!("`phone device boot {}`", quoted(&view.device.label)),
        (None, _) => format!("`phone device list` ranks what {host} has"),
    });

    said.join("; ")
}

async fn on_host(
    views: &[View],
    reg: &Registry,
    host: &str,
    kind: Option<Platform>,
    aim: Aim,
) -> Result<View> {
    let at = ssh::Where::of(Some(host));
    let pool = pool(views, host, kind);

    let (ledger, mine) = tokio::join!(lease::Leases::open(&at), lease::mine());
    let ledger = ledger.ok();
    let tree = mine.as_ref().map(|me| me.tree.as_str());

    let driven: BTreeMap<String, usage::Driven> = pool
        .iter()
        .map(|v| {
            let used = ledger.as_ref().map(|l| l.usage.of(&v.device, tree));

            (v.device.id.clone(), used.unwrap_or_default())
        })
        .collect();

    if aim == Aim::Bootable {
        let off: Vec<&View> = pool
            .iter()
            .copied()
            .filter(|v| v.reach == model::Reach::Off)
            .collect();

        return match usage::usual(&off, &driven) {
            Some((view, _)) => Ok(view.clone()),
            None => bail!("nothing on {host} was driven before; name one: `phone device list` ranks what {host} has"),
        };
    }

    let free = |v: &View| {
        let held = ledger
            .as_ref()
            .and_then(|l| l.holder(lease::key(&v.device)));

        held.is_none_or(|h| {
            mine.as_ref()
                .is_some_and(|me| h.admits(&me.tree, me.session.as_deref()))
        })
    };

    if let Some(view) = pick(&pool, &driven, free) {
        return Ok(view.clone());
    }

    let rooms = memory::rooms(reg, views, std::slice::from_ref(&at)).await;
    let room = rooms.into_iter().next().and_then(|(_, room)| room.ok());

    Err(Refused(stranded(host, &pool, &driven, room.as_ref())).into())
}

fn whose(holder: &lease::Holder, mine: Option<&lease::Holder>) -> String {
    match mine.is_some_and(|me| holder.admits(&me.tree, me.session.as_deref())) {
        true => format!("yours ({})", model::ago(holder.since)),
        false => holder.label(),
    }
}

fn print_table(
    views: &[View],
    holds: &BTreeMap<String, lease::Holder>,
    driven: &BTreeMap<String, usage::Driven>,
    mine: Option<&lease::Holder>,
) {
    if views.is_empty() {
        eprintln!("phone: nothing reachable or remembered");

        return;
    }

    for view in views {
        let d = &view.device;

        // two AVDs from one image carry one name, and so does a handset listed
        // beside the emulator named after it. The id is what a command can be
        // pointed at without a picker, so it is printed where a name is not
        // enough on its own.
        let name = listed_name(views, d);

        let endpoint = d
            .ranked_endpoints()
            .first()
            .map(|e| {
                let pin = e.pin.as_str();

                if pin.is_empty() {
                    e.addr()
                } else {
                    format!("{} [{pin}]", e.addr())
                }
            })
            // a device driven through a host has no address of its own; the host is it
            .or_else(|| d.host.clone())
            .unwrap_or_else(|| "-".into());

        let row = format!(
            "{:<12} {:<28} {:<20} {:<16} {:<24} {:<12} {}",
            format!("{} {}", d.platform.os(), d.platform.kind()),
            truncate(&name, 28),
            truncate(&d.model, 20),
            view.reach.label(),
            endpoint,
            driven.get(&d.id).copied().unwrap_or_default().label(),
            holds
                .get(&d.id)
                .map(|holder| whose(holder, mine))
                .unwrap_or_default(),
        );

        println!("{}", row.trim_end());
    }
}

fn listed_name(views: &[View], d: &model::Device) -> String {
    if views.iter().filter(|v| v.device.label == d.label).count() <= 1 {
        return d.label.clone();
    }

    if d.platform != Platform::Simulator {
        return d.id.clone();
    }

    let udid = d.id.rsplit('/').next().unwrap_or(&d.id);
    let short: String = udid.chars().take(8).collect();

    format!("{} {short}", truncate(&d.label, 19))
}

fn print_json(
    views: &[View],
    holds: &BTreeMap<String, lease::Holder>,
    driven: &BTreeMap<String, usage::Driven>,
) -> Result<()> {
    let rows: Vec<serde_json::Value> = views
        .iter()
        .map(|v| {
            serde_json::json!({
                "hold": holds.get(&v.device.id),
                "driven": driven.get(&v.device.id),
                "id": v.device.id,
                "label": v.device.label,
                "model": v.device.model,
                "platform": v.device.platform.as_str(),
                "os": v.device.platform.os(),
                "kind": v.device.platform.kind(),
                "reach": v.reach.label(),
                "serial": v.reach.serial(),
                "host": v.device.host,
                "discovered_id": v.device.discovered_id,
                "endpoints": v.device.endpoints,
                "last_connected": v.device.last_connected,
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }

    s.chars()
        .take(width.saturating_sub(1))
        .chain(['…'])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missed_wait_with_no_near_spelling_names_what_is_on_screen() {
        let labels = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();

        assert_eq!(
            on_screen(&labels(&["/ DAZZLE / HOSTS", "SSH", "TOOLS"])),
            "; on screen: / DAZZLE / HOSTS | SSH | TOOLS"
        );
        assert_eq!(
            on_screen(&labels(&["a", "b", "c", "d", "e", "f", "g", "h"])),
            "; on screen: a | b | c | d | e | f (+2 more)"
        );
        assert_eq!(
            on_screen(&labels(&["a", "", "b"])),
            "; on screen: a | b",
            "an unnamed row tells the agent nothing"
        );
        assert_eq!(on_screen(&[]), "; nothing on screen");
    }

    #[test]
    fn a_step_with_a_flag_of_another_verb_names_that_verb() {
        let err = |raw| parse_step(0, raw).err().unwrap().to_string();

        assert_eq!(
            err("wait Inbox --settle"),
            "step 1 (wait Inbox --settle): unexpected argument '--settle' found; --settle belongs to `shot`"
        );
        assert!(
            err("swipe up --timeout 3")
                .ends_with("; --timeout belongs to `up`, `wait`, `device boot`"),
            "an exact flag elsewhere beats clap's look-alike on this verb"
        );
        assert_eq!(
            err("tap OK --xyzzy"),
            "step 1 (tap OK --xyzzy): unexpected argument '--xyzzy' found"
        );
    }

    #[test]
    fn a_name_that_needs_quoting_is_handed_back_ready_to_paste() {
        assert_eq!(quoted("iPhone 17 Pro Max"), "\"iPhone 17 Pro Max\"");
        assert_eq!(quoted("pixel-9"), "pixel-9");
    }

    #[test]
    fn a_simulator_sharing_its_name_keeps_it_and_adds_a_typeable_udid() {
        let sim = |id: &str| {
            View::new(
                Device::new(id, "iPad (A16)", Platform::Simulator),
                Reach::Off,
            )
        };

        let views = vec![
            sim("rose/E6A29D48-6AD8-4F4B-9C6B-0A1B2C3D4E5F"),
            sim("rose/11111111-2222-3333-4444-555555555555"),
            emu("android_id:1111aaaa", "pixel_7-api36", Reach::Known),
            emu("avd:mac/pixel_7-api36", "pixel_7-api36", Reach::Off),
            emu("avd:mac/pixel_7-api36-c", "pixel_7-api36-c", Reach::Off),
        ];

        assert_eq!(listed_name(&views, &views[0].device), "iPad (A16) E6A29D48");
        assert_eq!(listed_name(&views, &views[2].device), "android_id:1111aaaa");
        assert_eq!(listed_name(&views, &views[4].device), "pixel_7-api36-c");
    }

    #[test]
    fn an_off_device_says_whether_booting_it_is_welcome() {
        let room = memory::Fit::Room("rose has room for it (6.3 GB left, it needs 2.5)".into());

        assert_eq!(
            off("pixel_7-api36", Some(room)),
            "pixel_7-api36 is off; rose has room for it (6.3 GB left, it needs 2.5): \
             `phone device boot pixel_7-api36`"
        );

        let full = memory::Fit::Full(vec![
            "no room on rose: this needs 2.5 GB and 0.4 is left".into()
        ]);
        let said = off("iPad (A16)", Some(full));

        assert!(said.starts_with("iPad (A16) is off and there is no room to boot it now: no room"));
        assert!(said.contains("`phone device boot \"iPad (A16)\" --over-budget`"));

        assert_eq!(
            off("pixel_7-api36", None),
            "pixel_7-api36 is off; start it with `phone device boot pixel_7-api36`"
        );
    }

    use model::{Device, Reach};

    fn mac() -> Server {
        Server::Remote {
            host: "mac".into(),
            port: 5038,
        }
    }

    fn emu(id: &str, label: &str, reach: Reach) -> View {
        View::new(Device::new(id, label, Platform::Emulator), reach).on(mac())
    }

    fn attached(serial: &str) -> Reach {
        Reach::Attached {
            serial: serial.into(),
            wireless: false,
        }
    }

    fn ids(views: &[View]) -> Vec<&str> {
        views.iter().map(|v| v.device.id.as_str()).collect()
    }

    fn wiped() -> Vec<View> {
        vec![
            emu(
                "android_id:1111aaaa",
                "pixel_7-api36",
                attached("emulator-5554"),
            ),
            emu("android_id:2222bbbb", "pixel_7-api36", Reach::Known),
            emu("android_id:3333cccc", "pixel_7-api36", Reach::Known),
            emu(
                "android_id:4444dddd",
                "pixel_7-api36-b",
                attached("emulator-5556"),
            ),
        ]
    }

    #[test]
    fn a_name_shared_with_rows_left_by_a_wipe_means_the_one_running() {
        let views = wiped();

        assert_eq!(
            ids(&candidates(&views, Some("pixel_7-api36"), Aim::Running)),
            ["android_id:1111aaaa"]
        );
    }

    #[test]
    fn booting_a_name_that_is_running_finds_it_running_rather_than_asking() {
        let views = wiped();

        assert_eq!(
            ids(&candidates(&views, Some("pixel_7-api36"), Aim::Bootable)),
            ["android_id:1111aaaa"]
        );
    }

    #[test]
    fn booting_a_name_a_running_handset_shares_starts_the_avd() {
        let handset = View::new(
            Device::new("SERIALNUMBER01", "pixel-9", Platform::Android),
            attached("SERIALNUMBER01"),
        );
        let avd = emu("avd:mac/pixel-9", "pixel-9", Reach::Off);

        let views = [handset, avd];

        assert_eq!(
            ids(&candidates(&views, Some("pixel-9"), Aim::Bootable)),
            ["avd:mac/pixel-9"]
        );
        assert_eq!(
            ids(&candidates(&views, Some("pixel-9"), Aim::Running)),
            ["SERIALNUMBER01"]
        );
    }

    #[test]
    fn two_running_devices_of_one_name_are_still_asked_about() {
        let views = [
            emu("android_id:1111", "sdk_gphone", attached("emulator-5554")),
            emu("android_id:2222", "sdk_gphone", attached("emulator-5556")),
        ];

        assert_eq!(
            candidates(&views, Some("sdk_gphone"), Aim::Running).len(),
            2
        );
    }

    #[test]
    fn a_serial_names_the_device_holding_it_on_any_host() {
        let views = wiped();

        for want in ["emulator-5556", "mac/emulator-5556"] {
            assert_eq!(
                ids(&candidates(&views, Some(want), Aim::Running)),
                ["android_id:4444dddd"],
                "{want}"
            );
        }
    }

    #[test]
    fn a_serial_still_filed_under_a_row_loses_to_the_device_holding_it() {
        let mut ghost = Device::new("android_id:5555eeee", "tablet-api35", Platform::Emulator);

        ghost.add_alias("emulator-5554");

        let views = [View::new(ghost, Reach::Known), wiped().remove(0)];

        assert_eq!(
            ids(&candidates(&views, Some("emulator-5554"), Aim::Running)),
            ["android_id:1111aaaa"]
        );
    }

    #[test]
    fn a_local_serial_typed_in_full_beats_the_same_serial_on_a_host() {
        let local = View::new(
            Device::new("emulator-5554", "remote-android", Platform::Emulator),
            attached("emulator-5554"),
        );
        let hosted = emu(
            "mac/emulator-5554",
            "remote-android",
            attached("emulator-5554"),
        );

        let views = [hosted, local];

        assert_eq!(
            ids(&candidates(&views, Some("emulator-5554"), Aim::Running)),
            ["emulator-5554"]
        );
    }

    #[test]
    fn a_substring_is_never_narrowed_to_whichever_is_running() {
        let views = wiped();

        assert_eq!(candidates(&views, Some("pixel"), Aim::Running).len(), 4);
    }

    use crate::usage::tests::on_rose;

    fn rose() -> Vec<View> {
        vec![
            on_rose("avd:pixel", "pixel", Platform::Emulator, Reach::Off),
            on_rose(
                "avd:tab",
                "tab",
                Platform::Emulator,
                attached("emulator-5554"),
            ),
            on_rose(
                "avd:fold",
                "fold",
                Platform::Emulator,
                attached("emulator-5556"),
            ),
            on_rose("AAAA", "iPhone 17", Platform::Simulator, Reach::Off),
        ]
    }

    fn used(project: u64, all: u64) -> usage::Driven {
        let of = |count| (count > 0).then_some(usage::Use { at: 1, count });

        usage::Driven {
            project: of(project),
            all: of(all),
        }
    }

    #[test]
    fn a_host_with_a_kind_names_its_devices_of_that_kind() {
        let views = rose();

        assert_eq!(host_target(&views, "ROSE"), Some(("rose".into(), None)));
        assert_eq!(
            host_target(&views, "rose/sim"),
            Some(("rose".into(), Some(Platform::Simulator)))
        );
        assert_eq!(host_target(&views, "rose/tablet"), None);
        assert_eq!(host_target(&views, "nyx"), None);
        assert_eq!(
            ids(&pool(&views, "rose", Some(Platform::Emulator))
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()),
            ["avd:pixel", "avd:tab", "avd:fold"]
        );
    }

    #[test]
    fn a_device_named_like_a_host_wins_over_the_host() {
        let mut views = rose();
        views.push(on_rose("avd:rose", "rose", Platform::Emulator, Reach::Off));

        assert_eq!(host_target(&views, "rose"), None);
        assert_eq!(host_target(&views, "emulator-5554"), None);
    }

    #[test]
    fn a_host_gives_the_most_driven_device_up_that_nobody_else_holds() {
        let views = rose();
        let pool = pool(&views, "rose", None);
        let driven = BTreeMap::from([
            ("avd:tab".to_string(), used(0, 40)),
            ("avd:fold".to_string(), used(2, 2)),
        ]);

        let got = pick(&pool, &driven, |_| true).map(|v| v.device.id.as_str());
        assert_eq!(got, Some("avd:fold"));

        let got = pick(&pool, &driven, |v| v.device.id != "avd:fold").map(|v| v.device.id.as_str());
        assert_eq!(got, Some("avd:tab"));

        assert!(pick(&pool, &driven, |_| false).is_none());
    }

    #[test]
    fn a_host_with_nothing_up_spells_out_what_to_boot() {
        let views = rose();
        let off: Vec<View> = views
            .iter()
            .filter(|v| v.reach == Reach::Off)
            .cloned()
            .collect();
        let pool = pool(&off, "rose", None);

        let driven = BTreeMap::from([("avd:pixel".to_string(), used(3, 9))]);
        assert_eq!(
            stranded("rose", &pool, &driven, None),
            "nothing on rose is up; this project usually boots pixel; `phone device boot pixel`"
        );

        let driven = BTreeMap::from([("AAAA".to_string(), used(0, 9))]);
        assert_eq!(
            stranded("rose", &pool, &driven, None),
            "nothing on rose is up; iPhone 17 is the most driven there; `phone device boot \"iPhone 17\"`"
        );

        let everyone = super::pool(&views, "rose", None);
        assert_eq!(
            stranded("rose", &everyone, &BTreeMap::new(), None),
            "everything up on rose is held by another project; `phone device list` ranks what rose has"
        );
    }
}
