mod app;
mod ascii_animation;
mod config;
mod greetd;
mod logs;
mod theme;
mod ui;

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use ratatui::crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::app::{Action, AppState, Effect};
use crate::config::{Cli, Config};
use crate::greetd::{spawn_mock, spawn_real, Channels};
use crate::logs::{spawn_logs, LogBuffer};
use crate::theme::Theme;
use crate::ui::Chrome;

enum Outcome {
    Launch,
    Quit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load(&cli)?;

    // Standalone showcase: full-screen animation, no config auth, no greetd.
    if cli.ascii_demo {
        let theme = Theme::resolve(cfg.accent, &cfg.overrides);
        let mut terminal = ratatui::init();
        let result = ascii_demo_loop(&mut terminal, &theme).await;
        ratatui::restore();
        return result;
    }

    run(&cli, cfg).await
}

async fn run(cli: &Cli, cfg: Config) -> Result<()> {
    let sock = std::env::var_os("GREETD_SOCK");

    // Real socket only when present AND not forced into demo; anything else runs
    // the mock, so launching from a plain terminal can never touch PAM.
    let (channels, demo) = match (&sock, cli.demo) {
        (Some(path), false) => (spawn_real(std::path::Path::new(path)).await?, false),
        (Some(_), true) => (spawn_mock(), true),
        (None, forced) => {
            if !forced {
                eprintln!("greeter: GREETD_SOCK unset — running in --demo mode (no real login).");
            }
            (spawn_mock(), true)
        }
    };

    let theme = Theme::resolve(cfg.accent, &cfg.overrides);
    let app = AppState::new(
        cfg.idle_status.clone(),
        cfg.default_user.clone(),
        cfg.session_cmd.clone(),
        demo,
    );
    let log_rx = spawn_logs(cfg.log_cmd.clone());

    let chrome = Chrome {
        theme: &theme,
        disclaimer: cfg.disclaimer.as_deref(),
        show_help: cfg.show_help,
    };

    let mut terminal = ratatui::init();
    let outcome = event_loop(&mut terminal, app, channels, log_rx, &chrome).await;
    ratatui::restore();

    match outcome? {
        Outcome::Launch => {
            if demo {
                println!(
                    "[demo] authentication OK — would exec: {}",
                    cfg.session_cmd.join(" ")
                );
            }
            // Real mode: exiting hands the VT to greetd, which starts the session.
        }
        Outcome::Quit => {}
    }
    Ok(())
}

async fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    mut app: AppState,
    greetd: Channels,
    mut log_rx: tokio::sync::mpsc::Receiver<String>,
    chrome: &Chrome<'_>,
) -> Result<Outcome> {
    let mut greetd = greetd;
    let mut logs = LogBuffer::new(500);
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(120));
    // Foreign writers (kernel/systemd) can paint over our frame; ratatui only
    // repaints cells it knows changed, so force a full repaint on the tick after
    // any log activity — adaptive noise-healing that stays quiet once boot settles.
    let mut logs_dirty = false;
    let mut force_clear = false;

    loop {
        if std::mem::take(&mut force_clear) {
            terminal.clear()?;
        }
        let mut scroll = app.log_scroll;
        terminal.draw(|f| scroll = ui::draw(f, &app, &logs, chrome))?;
        app.log_scroll = scroll;

        tokio::select! {
            _ = ticker.tick() => {
                app.update(Action::Tick);
                if logs_dirty {
                    force_clear = true;
                    logs_dirty = false;
                }
            }
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        if key.kind == KeyEventKind::Release {
                            continue;
                        }
                        // Ctrl+C: hard quit in demo, ignored in real mode.
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(key.code, KeyCode::Char('c'))
                        {
                            if app.demo {
                                return Ok(Outcome::Quit);
                            }
                            continue;
                        }
                        if let Some(action) = map_key(key.code, app.logs_open) {
                            let effects = app.update(action);
                            if let Some(outcome) = apply(effects, &greetd).await {
                                return Ok(outcome);
                            }
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => return Ok(Outcome::Quit),
                }
            }
            maybe_resp = greetd.resp_rx.recv() => {
                if let Some(resp) = maybe_resp {
                    let effects = app.update(Action::Greetd(resp));
                    if let Some(outcome) = apply(effects, &greetd).await {
                        return Ok(outcome);
                    }
                }
            }
            maybe_line = log_rx.recv() => {
                if let Some(line) = maybe_line {
                    logs.push(line);
                    logs_dirty = true;
                }
            }
        }
    }
}

fn map_key(code: KeyCode, logs_open: bool) -> Option<Action> {
    // While the log viewer is up, keys drive scrolling; nothing reaches the
    // credential fields, so plain letters are safe as scroll/close shortcuts.
    if logs_open {
        return match code {
            KeyCode::F(2) | KeyCode::Esc | KeyCode::Char('q') => Some(Action::ToggleLogs),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::ScrollLogs(1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::ScrollLogs(-1)),
            KeyCode::PageUp => Some(Action::ScrollLogs(10)),
            KeyCode::PageDown => Some(Action::ScrollLogs(-10)),
            KeyCode::Home => Some(Action::ScrollLogs(i32::MAX)),
            KeyCode::End => Some(Action::ScrollLogs(i32::MIN)),
            _ => None,
        };
    }
    match code {
        KeyCode::Enter => Some(Action::Submit),
        KeyCode::F(2) => Some(Action::ToggleLogs),
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Down | KeyCode::Up => Some(Action::FocusToggle),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Char(c) => Some(Action::Char(c)),
        _ => None,
    }
}

/// Hands-off showcase: the ascii animation runs full-screen forever with a
/// caption. No greetd, no auth — a phone-friendly demo.
async fn ascii_demo_loop<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    theme: &Theme,
) -> Result<()> {
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(120));
    let mut tick: u64 = 0;

    loop {
        terminal.draw(|f| ascii_demo_draw(f, tick, theme))?;

        tokio::select! {
            _ = ticker.tick() => tick = tick.wrapping_add(1),
            ev = events.next() => match ev {
                Some(Ok(Event::Key(k))) => {
                    if k.kind == KeyEventKind::Release {
                        continue;
                    }
                    let quit = matches!(k.code, KeyCode::Esc | KeyCode::Char('q'))
                        || (k.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(k.code, KeyCode::Char('c')));
                    if quit {
                        return Ok(());
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return Ok(()),
            }
        }
    }
}

fn ascii_demo_draw(f: &mut Frame, tick: u64, theme: &Theme) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);

    let [body, foot] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let sprite = ascii_animation::frame(tick, body.width, body.height, theme);
    f.render_widget(Paragraph::new(sprite), body);

    f.render_widget(
        Paragraph::new("ascii demo   ·   q / ESC quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.dim).add_modifier(Modifier::BOLD)),
        foot,
    );
}

async fn apply(effects: Vec<Effect>, greetd: &Channels) -> Option<Outcome> {
    for effect in effects {
        match effect {
            Effect::Send(req) => {
                // greetd task gone => session unrecoverable; bail.
                if greetd.req_tx.send(req).await.is_err() {
                    return Some(Outcome::Quit);
                }
            }
            Effect::LaunchAndExit => return Some(Outcome::Launch),
            Effect::Quit => return Some(Outcome::Quit),
        }
    }
    None
}
