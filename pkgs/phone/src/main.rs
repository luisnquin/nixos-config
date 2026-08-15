mod a11y;
mod actions;
mod adb;
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

use std::os::unix::process::CommandExt;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser};
use tokio::sync::mpsc::UnboundedReceiver;

use actions::{host_of, Sink};
use adb::Server;
use cli::{Cli, Command, HostAction};
use connect::{Reporter, Step};
use discover::survey;
use model::{Platform, View};
use registry::Registry;

#[tokio::main]
async fn main() -> ExitCode {
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

    match cli.command {
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
            let view = resolve(&mut reg, target.as_deref(), true).await?;

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

            let view = resolve(&mut reg, target.as_deref(), true).await?;

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
            let view = resolve(&mut reg, target.as_deref(), true).await?;

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

        Some(Command::Shot { target, out }) => {
            let view = resolve(&mut reg, target.as_deref(), true).await?;
            let sink = Sink::from_opt(out.as_deref());

            let (rep, drain) = reporter();
            let res = actions::screenshot(&view.server, &view.device, &sink, &rep).await;

            drop(rep);
            drain.await;

            eprintln!("phone: {}", res?);

            Ok(())
        }

        Some(Command::Logs { first, second }) => {
            let (target, app) = match second {
                Some(app) => (Some(first), app),
                None => (None, first),
            };

            let view = resolve(&mut reg, target.as_deref(), true).await?;
            let err = actions::logs_command(&view.server, &view.device, &app)
                .await?
                .exec();

            bail!("{err}");
        }

        Some(Command::Mirror { target }) => {
            let view = resolve(&mut reg, target.as_deref(), true).await?;

            eprintln!(
                "phone: {}",
                actions::mirror(&view.server, &view.device).await?
            );

            Ok(())
        }

        Some(Command::Install { apk, target }) => {
            let view = resolve(&mut reg, target.as_deref(), true).await?;

            let (rep, drain) = reporter();
            let res = actions::install(&view.server, &view.device, &apk, &rep).await;

            drop(rep);
            drain.await;

            eprintln!("phone: {}", res?);

            Ok(())
        }

        Some(Command::Hosts { action }) => hosts_cmd(&mut reg, action).await,

        Some(Command::Snapshot { target, json }) => {
            // uiautomator has no display flag; it reads whichever one holds focus
            let t = driven(&mut reg, target.as_deref(), cli.focus).await?;
            let nodes = a11y::dump(&t).await?;

            if json {
                print_elements_json(&nodes)?;
            } else {
                print_elements(&nodes);
            }

            Ok(())
        }

        Some(Command::Tap { what, target }) => {
            let t = driven(&mut reg, target.as_deref(), cli.focus).await?;

            // a literal point, for a game, a canvas, or an unfocused split half
            if let Ok((x, y)) = cli::parse_point(&what) {
                a11y::tap(&t, x, y).await?;
                eprintln!("phone: tapped {x},{y}");

                return Ok(());
            }

            let nodes = a11y::dump(&t).await?;
            let node = a11y::pick(&nodes, &what)?;
            let (x, y) = node.bounds.center();

            a11y::tap(&t, x, y).await?;
            eprintln!("phone: tapped {} at {x},{y}", node.label());

            Ok(())
        }

        Some(Command::Type { text, target }) => {
            let t = driven(&mut reg, target.as_deref(), cli.focus).await?;

            a11y::type_text(&t, &text).await?;
            eprintln!("phone: typed {} characters", text.chars().count());

            Ok(())
        }

        Some(Command::Key { name, target }) => {
            let t = driven(&mut reg, target.as_deref(), cli.focus).await?;

            a11y::key(&t, &name).await?;
            eprintln!("phone: sent {}", name.to_uppercase());

            Ok(())
        }

        Some(Command::Doctor) => doctor(&mut reg).await,

        Some(Command::Completions { .. }) => unreachable!("handled above"),
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

/// A device to read and press. `uiautomator` and `input` ride the adb transport,
/// so a handset here and an emulator on a mac behave alike. A simulator has no
/// transport at all — CoreSimulator is macOS-local — so its verbs run on the
/// host, which needs a `phone` of its own. An iPhone has neither.
async fn driven(
    reg: &mut Registry,
    want: Option<&str>,
    focus: Option<(i32, i32)>,
) -> Result<a11y::Target> {
    let view = resolve(reg, want, true).await?;

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
        server: view.server,
        serial,
        focus,
    }))
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
            "{:<9} {:<20} {:<20} {:<16} {:<24} {}",
            d.platform.as_str(),
            truncate(&d.label, 20),
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
