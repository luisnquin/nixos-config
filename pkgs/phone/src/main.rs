mod a11y;
mod actions;
mod adb;
mod avd;
mod cli;
mod connect;
mod discover;
mod hosts;
mod ios;
mod model;
mod picker;
mod registry;
mod simctl;
mod ssh;
mod tui;
use std::time::Duration;

use std::os::unix::process::CommandExt;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser};
use tokio::sync::mpsc::UnboundedReceiver;

use actions::{host_of, Sink};
use adb::Server;
use cli::{Cli, Command, HostAction, DEFAULT_AMOUNT};
use connect::{Reporter, Step};
use discover::survey;
use model::{Platform, View};
use registry::Registry;

#[tokio::main]
async fn main() -> ExitCode {
    // Rust ignores SIGPIPE, so a write to a closed pipe comes back as an error
    // that `println!` panics on. `phone snapshot | head` is how a long dump is
    // read, and it must end the run quietly rather than in a backtrace.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };

    match dispatch(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("phone: {e}");

            ExitCode::FAILURE
        }
    }
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
        let session = Session::open(&mut reg, want.as_deref(), focus).await?;

        return step(&session, command.expect("classified as a screen verb")).await;
    }

    match command {
        None => {
            if let Some(mut cmd) = tui::run(reg).await? {
                let err = cmd.exec();

                bail!("{err}");
            }

            Ok(())
        }

        Some(Command::Devices { json }) => {
            let views = survey(&mut reg).await;
            reg.save()?;

            if json {
                print_json(&views)?;
            } else {
                print_table(&views);
            }

            Ok(())
        }

        Some(Command::Connect {
            target,
            no_sweep,
            range,
            concurrency,
        }) => {
            let view = resolve(&mut reg, want(target).as_deref(), true).await?;

            let opts = connect::Opts {
                sweep: !no_sweep,
                range: range.unwrap_or(discover::sweep::EPHEMERAL),
                concurrency,
            };

            let (rep, drain) = reporter();
            let serial = connect::connect(&mut reg, &view.server, &view.device, &opts, &rep).await;

            drop(rep);
            drain.await;

            println!("{}", serial?);

            Ok(())
        }

        Some(Command::Disconnect { target, all }) => {
            if all {
                adb::run(&Server::Local, &["disconnect"]).await?;
                eprintln!("phone: dropped every wireless transport");

                return Ok(());
            }

            let view = resolve(&mut reg, want(target).as_deref(), true).await?;

            let Some(serial) = view.reach.serial().filter(|s| s.contains(':')) else {
                bail!("{} has no wireless transport", view.device.label);
            };

            adb::disconnect(&view.server, serial).await?;
            eprintln!("phone: disconnected {serial}");

            Ok(())
        }

        Some(Command::Pair { code, addr }) => {
            let (rep, drain) = reporter();
            let res = connect::pair(&Server::Local, addr.as_deref(), &code, &rep).await;

            drop(rep);
            drain.await;

            res?;

            eprintln!("phone: now run `phone connect` to bring the transport up");

            Ok(())
        }

        Some(Command::Pin { target, port }) => {
            let view = resolve(&mut reg, want(target).as_deref(), true).await?;

            let (rep, drain) = reporter();
            let res = connect::pin(&mut reg, &view.server, &view.device, port, &rep).await;

            drop(rep);
            drain.await;

            res
        }

        Some(Command::Use { target }) => {
            let view = resolve(&mut reg, target.as_deref(), false).await?;

            reg.current = Some(view.device.id.clone());
            reg.save()?;

            eprintln!("phone: default target is {}", view.device.label);

            Ok(())
        }

        Some(Command::Forget { target }) => {
            let matches = reg.find(&target);

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

        Some(Command::Logs { app }) => {
            let view = resolve(&mut reg, want(None).as_deref(), true).await?;
            let err = actions::logs_command(&view.server, &view.device, &app)
                .await?
                .exec();

            bail!("{err}");
        }

        Some(Command::Mirror { target }) => {
            let view = resolve(&mut reg, want(target).as_deref(), true).await?;

            eprintln!(
                "phone: {}",
                actions::mirror(&view.server, &view.device).await?
            );

            Ok(())
        }

        Some(Command::Install { apk }) => {
            let view = resolve(&mut reg, want(None).as_deref(), true).await?;

            let (rep, drain) = reporter();
            let res = actions::install(&view.server, &view.device, &apk, &rep).await;

            drop(rep);
            drain.await;

            eprintln!("phone: {}", res?);

            Ok(())
        }

        Some(Command::Boot { target, timeout }) => {
            let view = resolve(&mut reg, want(target).as_deref(), true).await?;

            boot(&mut reg, view, timeout).await
        }

        Some(Command::Shutdown { target }) => {
            let view = resolve(&mut reg, want(target).as_deref(), true).await?;

            eprintln!("phone: {}", actions::stop(&view.device, &view.reach).await?);

            Ok(())
        }

        Some(Command::Hosts { action }) => hosts_cmd(&mut reg, action).await,

        Some(Command::Doctor) => doctor(&mut reg).await,

        Some(Command::Completions { .. }) => unreachable!("handled above"),

        // every screen verb returned above, where it was given a device
        Some(_) => unreachable!("a screen verb reached dispatch"),
    }
}

/// Lists or toggles the ssh hosts a survey reaches into. The names come from
/// ssh, and how to reach one is already answered by the user's `ssh_config`.
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

        Some(HostAction::Disable { name }) => {
            reg.host_mut(&name).enabled = false;
            reg.save()?;

            eprintln!("phone: {name} disabled");

            Ok(())
        }

        None => {
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
}

impl Session {
    async fn open(
        reg: &mut Registry,
        want: Option<&str>,
        focus: Option<(i32, i32)>,
    ) -> Result<Self> {
        let view = resolve(reg, want, true).await?;
        let target = target_of(&view, focus).await?;

        Ok(Session {
            view,
            target,
            focus,
        })
    }
}

/// A screen verb against an already-resolved device.
async fn step(s: &Session, command: Command) -> Result<()> {
    match command {
        Command::Shot {
            target,
            out,
            crop,
            pad,
            scale,
            jpeg,
            settle,
        } => {
            let _ = target;

            // a frame is the whole display whatever holds focus, so --focus only
            // reaches the dump that resolves --crop
            if s.focus.is_some() && crop.is_none() {
                bail!("--focus picks the window a dump reads; a frame has no window");
            }

            // reading the frame and reading the elements in it are two calls to
            // the same device, so the crop is worked out off this one view
            let crop = match &crop {
                Some(spec) => Some(crop_bounds(&s.target, spec, pad).await?),
                None => None,
            };

            let sink = Sink::from_opt(out.as_deref());
            let shot = actions::Shot {
                crop,
                scale,
                jpeg,
                settle,
            };

            let (rep, drain) = reporter();
            let res = actions::screenshot(&s.view.server, &s.view.device, &sink, &rep, &shot).await;

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
            let t = &s.target;
            let nodes = a11y::dump(t).await?;

            if json {
                print_elements_json(&nodes)?;
            } else {
                print_elements(&nodes);
            }

            Ok(())
        }

        Command::Tap { what } => {
            let t = &s.target;
            let ((x, y), name) = at(t, &what).await?;

            a11y::tap(t, x, y).await?;
            eprintln!("phone: tapped {}", aim((x, y), name));

            Ok(())
        }

        Command::Press { what, hold } => {
            let t = &s.target;
            let ((x, y), name) = at(t, &what).await?;

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

        Command::Swipe {
            from,
            to,
            duration,
            amount,
        } => {
            let t = &s.target;

            let (from, to) = match &to {
                Some(to) => {
                    // the two forms answer the same question differently, and a
                    // flag that belongs to the other one is a misunderstanding
                    // worth reporting rather than dropping
                    if amount != DEFAULT_AMOUNT {
                        bail!("--amount sizes a directional swipe; this one has both ends");
                    }

                    (at(t, &from).await?.0, at(t, to).await?.0)
                }
                None => {
                    let direction = from.parse().map_err(|e| {
                        anyhow::anyhow!("{e}; a swipe from a point needs somewhere to go")
                    })?;

                    a11y::along(a11y::size(t).await?, direction, amount)
                }
            };

            a11y::swipe(t, from, to, duration).await?;
            eprintln!(
                "phone: swiped {},{} to {},{} over {}ms",
                from.0,
                from.1,
                to.0,
                to.1,
                duration.as_millis()
            );

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

            wait(t, &what, gone, timeout).await
        }

        Command::Type { text } => {
            let t = &s.target;

            a11y::type_text(t, &text).await?;
            eprintln!("phone: typed {} characters", text.chars().count());

            Ok(())
        }

        Command::Key { name } => {
            let t = &s.target;

            a11y::key(t, &name).await?;
            eprintln!("phone: sent {}", name.to_uppercase());

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
    for (n, raw) in steps.iter().enumerate() {
        let words = shell_words::split(raw).map_err(|e| anyhow::anyhow!("step {}: {e}", n + 1))?;

        let parsed = Cli::try_parse_from(std::iter::once("phone".to_string()).chain(words))
            .map_err(|e| {
                anyhow::anyhow!("step {} ({raw}): {}", n + 1, first_line(&e.to_string()))
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

        // named before it runs, not after: a step that hangs is the one an agent
        // needs to see, and its own line only arrives once it is over
        eprintln!("  [{}/{}] {raw}", n + 1, steps.len());

        Box::pin(step(s, command))
            .await
            .map_err(|e| anyhow::anyhow!("step {} ({raw}): {e}", n + 1))?;
    }

    Ok(())
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
            host: host_of(&view.device)?.to_string(),
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

    let serial = connect::attached_serial(&view.server, &view.device)
        .await
        .ok_or_else(|| anyhow::anyhow!("{} is not attached", view.device.label))?;

    Ok(a11y::Target::Adb(a11y::Adb {
        display: adb::active_display(&view.server, &serial).await,
        server: view.server.clone(),
        serial,
        focus,
    }))
}

/// A point to aim at, and what it turned out to be. `X,Y` is taken literally —
/// a canvas, a map, an unfocused split half — and anything else names an
/// element. The name comes back so that a tap, a hold and a drag all report the
/// same way; a caller that resolved an element wants to see which one.
async fn at(t: &a11y::Target, what: &str) -> Result<((i32, i32), Option<String>)> {
    if let Ok(point) = cli::parse_point(what) {
        return Ok((point, None));
    }

    let nodes = a11y::dump(t).await?;
    let node = a11y::pick(&nodes, what)?;

    Ok((node.bounds.center(), Some(node.label().to_string())))
}

/// `at 416,1627` for a bare point, `Gmail at 416,1627` for a named element.
fn aim(point: (i32, i32), name: Option<String>) -> String {
    match name {
        Some(name) => format!("{name} at {},{}", point.0, point.1),
        None => format!("{},{}", point.0, point.1),
    }
}

/// The part of the frame to keep, in pixels. An element is padded because a
/// crop tight to its bounds shows a control with nothing around it to say where
/// on the screen it is; an explicit rectangle is taken as given.
async fn crop_bounds(t: &a11y::Target, spec: &str, pad: i32) -> Result<a11y::Bounds> {
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
            let nodes = a11y::dump(t).await?;

            a11y::pick(&nodes, spec)?.bounds.padded(pad, Some(panel))
        }
    };

    Ok(size.in_pixels(bounds))
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
) -> Result<()> {
    let started = std::time::Instant::now();

    loop {
        // a dump that fails mid-transition is not an answer either way
        let present = a11y::dump(t)
            .await
            .map(|nodes| a11y::present(&nodes, what))
            .unwrap_or(gone);

        if present != gone {
            eprintln!(
                "phone: '{what}' {} after {:.1}s",
                if gone { "left" } else { "appeared" },
                started.elapsed().as_secs_f64()
            );

            return Ok(());
        }

        if started.elapsed() >= timeout {
            bail!(
                "'{what}' was still {} after {:.0}s",
                if gone { "there" } else { "missing" },
                timeout.as_secs_f64()
            );
        }

        tokio::time::sleep(POLL).await;
    }
}

fn print_elements(nodes: &[a11y::Node]) {
    for node in nodes {
        let (x, y) = node.bounds.center();
        let press = if node.clickable { "tap" } else { "   " };

        println!("@{:<3} {press}  {:<40} {x},{y}", node.index, node.label());
    }
}

fn print_elements_json(nodes: &[a11y::Node]) -> Result<()> {
    let rows: Vec<serde_json::Value> = nodes
        .iter()
        .map(|node| {
            let (x, y) = node.bounds.center();

            serde_json::json!({
                "ref": format!("@{}", node.index),
                "label": node.label(),
                "text": node.text,
                "desc": node.desc,
                "id": node.res_id,
                "clickable": node.clickable,
                "at": [x, y],
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}

/// Turns whatever the user typed into exactly one device. `prefer_recent` is
/// what makes a bare `phone connect` one keystroke: with nothing to go on it
/// reaches for the last device used rather than a mostly-offline picker.
/// The device has to be surveyed again once it is up: the survey is what opens
/// the forward to its host's adb server, so a device that just booted is not
/// yet one the next command can reach.
async fn boot(reg: &mut Registry, view: View, timeout: Duration) -> Result<()> {
    if actions::running(&view.reach) {
        eprintln!("phone: {} is already running", view.device.label);

        return Ok(());
    }

    let label = view.device.label.clone();

    let (rep, drain) = reporter();
    let res = actions::boot(&view.device, timeout, &rep).await;

    drop(rep);
    drain.await;

    eprintln!("phone: {}", res?);

    let found = survey(reg).await;
    reg.save()?;

    match found
        .iter()
        .find(|v| v.device.is(&label) && actions::running(&v.reach))
    {
        Some(v) => println!("{} is {}", v.device.label, v.reach.label()),
        None => bail!("{label} booted but no survey can see it yet"),
    }

    Ok(())
}

async fn resolve(reg: &mut Registry, want: Option<&str>, prefer_recent: bool) -> Result<View> {
    let views = survey(reg).await;
    reg.save()?;

    let want = want
        .map(str::to_string)
        .or_else(|| std::env::var("PHONE_TARGET").ok())
        .filter(|s| !s.is_empty());

    let mut candidates: Vec<View> = match &want {
        Some(w) => {
            let exact: Vec<View> = views.iter().filter(|v| v.device.is(w)).cloned().collect();

            if exact.is_empty() {
                views
                    .iter()
                    .filter(|v| v.device.matches(w))
                    .cloned()
                    .collect()
            } else {
                exact
            }
        }
        None => views.clone(),
    };

    if candidates.is_empty() {
        bail!(match want {
            Some(w) => format!("no device matching '{w}'"),
            None => "no device reachable (check: adb devices, tailscale status)".to_string(),
        });
    }

    if want.is_none() {
        if let Some(id) = reg.current.clone() {
            if let Some(view) = candidates.iter().find(|v| v.device.id == id) {
                return Ok(view.clone());
            }
        }

        if prefer_recent {
            let recent = candidates
                .iter()
                .filter(|v| v.device.last_connected.is_some())
                .max_by_key(|v| v.device.last_connected.unwrap_or(0));

            if let Some(view) = recent {
                return Ok(view.clone());
            }
        }
    }

    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }

    let index = picker::pick(&candidates, "phone").await?;

    Ok(candidates.remove(index))
}

fn print_table(views: &[View]) {
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
        let name = if views.iter().filter(|v| v.device.label == d.label).count() > 1 {
            d.id.clone()
        } else {
            d.label.clone()
        };

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

        println!(
            "{:<9} {:<28} {:<20} {:<16} {:<24} {}",
            d.platform.as_str(),
            truncate(&name, 28),
            truncate(&d.model, 20),
            view.reach.label(),
            endpoint,
            d.last_connected
                .map(model::ago)
                .unwrap_or_else(|| "never".into()),
        );
    }
}

fn print_json(views: &[View]) -> Result<()> {
    let rows: Vec<serde_json::Value> = views
        .iter()
        .map(|v| {
            serde_json::json!({
                "id": v.device.id,
                "label": v.device.label,
                "model": v.device.model,
                "platform": v.device.platform.as_str(),
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

async fn doctor(reg: &mut Registry) -> Result<()> {
    let mut bad = 0;

    let mut check = |ok: bool, name: &str, detail: String| {
        if ok {
            println!("  ✓ {name:<16} {detail}");
        } else {
            bad += 1;
            println!("  ✗ {name:<16} {detail}");
        }
    };

    let adb_version = adb::run(&Server::Local, &["version"]).await;

    check(
        adb_version.as_ref().is_ok_and(|o| o.ok()),
        "adb",
        adb_version
            .as_ref()
            .ok()
            .and_then(|o| o.stdout.lines().next().map(str::to_string))
            .unwrap_or_else(|| "not on PATH".into()),
    );

    let attached = adb::devices(&Server::Local).await.unwrap_or_default();

    check(true, "transports", format!("{} attached", attached.len()));

    let key = registry::state_dir()
        .parent()
        .map(|_| dirs_adbkey())
        .unwrap_or_default();

    check(
        key.exists(),
        "adbkey",
        if key.exists() {
            key.display().to_string()
        } else {
            "missing; adb will generate one on first use".into()
        },
    );

    let peers = discover::tailscale::peers().await;

    match &peers {
        Ok(peers) => {
            let android = peers.iter().filter(|p| p.is_android()).count();
            let online = peers.iter().filter(|p| p.is_android() && p.online).count();

            check(
                true,
                "tailscale",
                format!("{android} android peer(s), {online} online"),
            );
        }
        Err(e) => check(false, "tailscale", e.to_string()),
    }

    // adb from nixpkgs is built without the bundled mDNS responder, so wireless
    // pairing depends entirely on the system's avahi
    let mdns = adb::run(&Server::Local, &["mdns", "check"]).await;
    let adb_mdns = mdns
        .as_ref()
        .is_ok_and(|o| !o.stderr.contains("not supported"));

    check(
        adb_mdns || which("avahi-browse"),
        "mdns",
        if adb_mdns {
            "adb has its own responder".into()
        } else if which("avahi-browse") {
            "via avahi-browse (adb has no responder)".into()
        } else {
            "no adb responder and no avahi-browse; pairing needs a manual addr".into()
        },
    );

    for tool in ["fzf", "scrcpy", "wl-copy", "notify-send"] {
        check(
            which(tool),
            tool,
            if which(tool) {
                "ok".into()
            } else {
                "not on PATH".into()
            },
        );
    }

    let known = hosts::discover().await;
    reg.sync_hosts(&known.iter().map(|h| h.name.clone()).collect::<Vec<_>>());

    let enabled: Vec<String> = reg
        .enabled_hosts()
        .iter()
        .map(|h| format!("{} ({})", h.name, h.caps.label()))
        .collect();

    check(
        true,
        "ssh hosts",
        if enabled.is_empty() {
            format!(
                "{} in your ssh config, none enabled; `phone hosts enable NAME`",
                known.len()
            )
        } else {
            enabled.join(", ")
        },
    );

    for state in reg
        .hosts
        .iter()
        .filter(|h| h.enabled)
        .cloned()
        .collect::<Vec<_>>()
    {
        match hosts::probe(&state.name).await {
            Some(caps) if caps == state.caps => {
                check(true, &state.name, format!("still {}", caps.label()))
            }
            Some(caps) => check(
                false,
                &state.name,
                format!("now {} (was {})", caps.label(), state.caps.label()),
            ),
            None => check(false, &state.name, "unreachable over ssh".into()),
        }
    }

    reg.save()?;

    if bad > 0 {
        bail!("{bad} check(s) failed");
    }

    Ok(())
}

fn dirs_adbkey() -> std::path::PathBuf {
    std::env::var_os("ANDROID_VENDOR_KEYS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".android/adbkey")
        })
}

fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}
