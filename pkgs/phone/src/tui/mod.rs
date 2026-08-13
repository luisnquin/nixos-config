pub mod app;
pub mod ui;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use tokio::sync::Mutex;

use crate::registry::Registry;
use app::{App, Mode, Outcome, Prompt};

/// Returns a command the caller must run after the terminal is back to normal;
/// `phone logs` is the only action that cannot coexist with the UI.
pub async fn run(reg: Registry) -> Result<Option<std::process::Command>> {
    let current = reg.current.clone();
    let shared = Arc::new(Mutex::new(reg));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut app = App::new(shared, current, tx);

    app.refresh();

    let mut terminal = ratatui::init();
    let mut events = EventStream::new();

    let period = Duration::from_secs(8);
    let mut tick = tokio::time::interval_at(tokio::time::Instant::now() + period, period);

    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let outcome = loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        tokio::select! {
            Some(Ok(event)) = events.next() => on_event(&mut app, event),
            Some(msg) = rx.recv() => app.on_msg(msg),
            _ = tick.tick() => app.refresh(),
        }

        if let Some(outcome) = app.quit.take() {
            break outcome;
        }
    };

    ratatui::restore();

    Ok(match outcome {
        Outcome::Quit => None,
        Outcome::Exec(cmd) => Some(cmd),
    })
}

fn on_event(app: &mut App, event: Event) {
    let Event::Key(key) = event else { return };

    if key.kind != KeyEventKind::Press {
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    match app.mode {
        Mode::Help => app.mode = Mode::Normal,
        Mode::Hosts => on_hosts_key(app, key),
        Mode::Filter => on_filter_key(app, key),
        Mode::Prompt(prompt) => on_prompt_key(app, key, prompt),
        Mode::Normal => on_normal_key(app, key),
    }
}

fn on_hosts_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => app.mode = Mode::Normal,
        KeyCode::Char('j') | KeyCode::Down => app.move_host_by(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_host_by(-1),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_host(),
        KeyCode::Char('r') => app.scan_hosts(),
        KeyCode::Char('a') => {
            app.input.clear();
            app.mode = Mode::Prompt(Prompt::Host);
        }
        _ => {}
    }
}

fn on_filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.filter.clear();
            app.mode = Mode::Normal;
            app.refilter();
        }
        KeyCode::Enter => app.mode = Mode::Normal,
        KeyCode::Backspace => {
            app.filter.pop();
            app.refilter();
        }
        KeyCode::Down => app.move_by(1),
        KeyCode::Up => app.move_by(-1),
        KeyCode::Char(c) => {
            app.filter.push(c);
            app.refilter();
        }
        _ => {}
    }
}

fn on_prompt_key(app: &mut App, key: KeyEvent, prompt: Prompt) {
    match key.code {
        KeyCode::Esc => {
            app.input.clear();

            app.mode = match prompt {
                Prompt::Host => Mode::Hosts,
                _ => Mode::Normal,
            };
        }
        KeyCode::Enter => app.submit_prompt(prompt),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => app.input.push(c),
        _ => {}
    }
}

fn on_normal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
        KeyCode::Char('j') | KeyCode::Down => app.move_by(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_by(-1),
        KeyCode::Char('g') | KeyCode::Home => app.select_edge(false),
        KeyCode::Char('G') | KeyCode::End => app.select_edge(true),
        KeyCode::Enter | KeyCode::Char('c') => app.connect_selected(),
        KeyCode::Char('d') => app.disconnect_selected(),
        KeyCode::Char('s') => app.shot_selected(),
        KeyCode::Char('m') => app.mirror_selected(),
        KeyCode::Char('p') => app.pin_selected(),
        KeyCode::Char('u') => app.use_selected(),
        KeyCode::Char('x') => app.forget_selected(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('l') => {
            app.input.clear();
            app.mode = Mode::Prompt(Prompt::Logs);
        }
        KeyCode::Char('P') => {
            app.input.clear();
            app.mode = Mode::Prompt(Prompt::PairCode);
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Filter;
        }
        KeyCode::Char('h') => app.open_hosts(),
        KeyCode::Char('?') => app.mode = Mode::Help,
        _ => {}
    }
}
