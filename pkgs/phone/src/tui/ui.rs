use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{row_fields, App, Level, Mode, Prompt, Scan};
use crate::model::Reach;

const KEYS: &[(&str, &str)] = &[
    ("enter/c", "connect"),
    ("d", "disconnect"),
    ("s", "screenshot (queues if away)"),
    ("m", "mirror"),
    ("l", "logs"),
    ("p", "pin 5555"),
    ("P", "pair"),
    ("u", "use as default"),
    ("x", "forget"),
    ("r", "refresh"),
    ("h", "ssh hosts"),
    ("/", "filter"),
    ("g/G", "top/bottom"),
    ("?", "help"),
    ("q", "quit"),
];

const SIDE_BY_SIDE_MIN_WIDTH: u16 = 100;
const SIDE_BY_SIDE_MIN_ASPECT: u16 = 3;
const DEVICE_MIN_WIDTH: u16 = 60;
const ACTIVITY_WIDTH: u16 = 38;
const ACTIVITY_HEIGHT: u16 = 5;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_PERIOD_MS: u128 = 90;

fn scanning_label(scan: &Scan, width: u16) -> String {
    let i = (scan.since.elapsed().as_millis() / SPINNER_PERIOD_MS) as usize % SPINNER.len();
    let spin = SPINNER[i];

    if scan.pending.is_empty() {
        return format!(" {spin} scanning ");
    }

    let names = scan.pending.join(" · ");

    if names.chars().count() + 14 <= width as usize {
        format!(" {spin} scanning {names} ")
    } else {
        format!(" {spin} scanning {} sources ", scan.pending.len())
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let [header, content, status, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let [list, log] = content_areas(content);

    render_header(frame, app, header);
    render_list(frame, app, list);
    render_log(frame, app, log);
    render_status(frame, app, status);
    render_footer(frame, footer, app);

    if matches!(app.mode, Mode::Help) {
        render_help(frame, frame.area());
    }

    if matches!(app.mode, Mode::Hosts | Mode::Prompt(Prompt::Host)) {
        render_hosts(frame, app, frame.area());
    }
}

fn content_areas(area: Rect) -> [Rect; 2] {
    if area.width >= SIDE_BY_SIDE_MIN_WIDTH
        && area.width >= area.height.saturating_mul(SIDE_BY_SIDE_MIN_ASPECT)
    {
        let [devices, _, activity] = Layout::horizontal([
            Constraint::Min(DEVICE_MIN_WIDTH),
            Constraint::Length(1),
            Constraint::Length(ACTIVITY_WIDTH),
        ])
        .areas(area);

        [devices, activity]
    } else {
        Layout::vertical([Constraint::Min(5), Constraint::Length(ACTIVITY_HEIGHT)]).areas(area)
    }
}

/// The ssh hosts a survey may reach into. A row is only a name plus whether it
/// is worth the round trip; how to reach it stays ssh's business.
fn render_hosts(frame: &mut Frame, app: &mut App, area: Rect) {
    let width = 54;
    let height = (app.hosts.len() as u16).clamp(1, 14) + 3;

    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);

    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);

    let block = Block::bordered()
        .title(" ssh hosts ")
        .title_bottom(" space toggle · a add · r rescan · esc close ");

    frame.render_widget(Clear, area);

    if app.hosts.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  reading ssh config…",
                Style::default().fg(Color::DarkGray),
            )))
            .block(block),
            area,
        );

        return;
    }

    let rows: Vec<ListItem> = app
        .hosts
        .iter()
        .map(|host| {
            let (mark, mark_style) = if host.enabled {
                ("[x] ", Style::default().fg(Color::Green))
            } else {
                ("[ ] ", Style::default().fg(Color::DarkGray))
            };

            // unprobed is not the same as answered and driving nothing
            let (caps, caps_color) = match (host.probed, host.caps.any()) {
                (false, _) => (String::from("unprobed"), Color::DarkGray),
                (true, false) => (host.caps.label(), Color::Red),
                (true, true) => (host.caps.label(), Color::Cyan),
            };

            ListItem::new(Line::from(vec![
                Span::styled(mark, mark_style),
                Span::styled(fit(&host.name, 20), Style::default().fg(Color::White)),
                Span::styled(caps, Style::default().fg(caps_color)),
            ]))
        })
        .collect();

    let list = List::new(rows)
        .block(block)
        .highlight_symbol("▸ ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut app.host_state);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(
            " phone ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(app.header(), Style::default().fg(Color::DarkGray)),
    ];

    if let Some(label) = app.current_label() {
        spans.push(Span::styled(
            "   default: ",
            Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::styled(label, Style::default().fg(Color::Magenta)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut block = Block::bordered().title(" devices ");

    if let Some(scan) = &app.scan {
        block = block.title_bottom(Span::styled(
            scanning_label(scan, area.width),
            Style::default().fg(Color::Yellow),
        ));
    }

    let inner = block.inner(area);

    let (w_label, w_model, w_reach) = columns(inner.width);

    let rows: Vec<ListItem> = app
        .visible
        .iter()
        .filter_map(|i| app.views.get(*i))
        .map(|view| {
            let (label, model, reach, detail) = row_fields(view);

            ListItem::new(Line::from(vec![
                Span::styled(
                    fit(&label, w_label),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(fit(&model, w_model), Style::default().fg(Color::Gray)),
                Span::styled(fit(&reach, w_reach), reach_style(&view.reach)),
                Span::styled(detail, detail_style(view.device.host.as_deref())),
            ]))
        })
        .collect();

    let list = List::new(rows)
        .block(block)
        .highlight_symbol("▸ ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut app.state);
}

/// Colours the where-it-lives column by owning machine, so a local emulator and
/// one on a mac are told apart at a glance. Grey means this machine. Derived
/// from the name rather than assigned, because hosts come out of ssh's config
/// and two must not collapse into one reading.
fn detail_style(host: Option<&str>) -> Style {
    const PALETTE: &[Color] = &[
        Color::Magenta,
        Color::Blue,
        Color::Yellow,
        Color::Green,
        Color::LightRed,
        Color::LightCyan,
    ];

    let Some(host) = host else {
        return Style::default().fg(Color::DarkGray);
    };

    let sum = host.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });

    Style::default().fg(PALETTE[sum % PALETTE.len()])
}

fn reach_style(reach: &Reach) -> Style {
    let color = match reach {
        Reach::Attached { .. } => Color::Green,
        Reach::Online => Color::Cyan,
        Reach::Unauthorized { .. } => Color::Yellow,
        Reach::Off => Color::Blue,
        Reach::Known => Color::DarkGray,
        Reach::Offline { .. } => Color::Red,
    };

    Style::default().fg(color)
}

fn render_log(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered().title(" activity ");
    let height = block.inner(area).height as usize;

    let lines: Vec<Line> = app
        .log
        .iter()
        .rev()
        .take(height)
        .rev()
        .map(|entry| {
            let (marker, color) = match entry.level {
                Level::Try => ("· ", Color::Blue),
                Level::Done => ("✓ ", Color::Green),
                Level::Fail => ("✗ ", Color::Red),
                Level::Note => ("• ", Color::Yellow),
            };

            Line::from(vec![
                Span::styled(marker, Style::default().fg(color)),
                Span::raw(entry.text.clone()),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    match &app.mode {
        Mode::Filter => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("/", Style::default().fg(Color::Cyan)),
                    Span::raw(app.filter.clone()),
                    Span::styled("_", Style::default().fg(Color::DarkGray)),
                ])),
                area,
            );

            return;
        }
        Mode::Prompt(prompt) => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!("{}: ", prompt.label()),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(app.input.clone()),
                    Span::styled("_", Style::default().fg(Color::DarkGray)),
                ])),
                area,
            );

            return;
        }
        _ => {}
    }

    // a port sweep is the one step long enough that a spinner is not enough
    if let (Some(busy), Some((done, total))) = (&app.busy, app.progress) {
        let ratio = done as f64 / total.max(1) as f64;

        frame.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(Color::Cyan))
                .ratio(ratio.clamp(0.0, 1.0))
                .label(format!("{busy}  {done}/{total}")),
            area,
        );

        return;
    }

    let line = match (&app.busy, app.queued_label()) {
        (Some(busy), _) => Line::from(Span::styled(
            format!(" {busy}…"),
            Style::default().fg(Color::Cyan),
        )),
        (None, Some(queued)) => {
            let why = app
                .queued_hint()
                .map(|why| format!(" · {why}"))
                .unwrap_or_default();

            Line::from(Span::styled(
                format!(" waiting to shoot {queued}{why}"),
                Style::default().fg(Color::Yellow),
            ))
        }
        _ if !app.filter.is_empty() => Line::from(Span::styled(
            format!(" filter: {}", app.filter),
            Style::default().fg(Color::DarkGray),
        )),
        _ => Line::from(""),
    };

    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hints = app.hints();

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {}", hints.join("   ")),
            Style::default().fg(Color::DarkGray),
        ))),
        area,
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let width = 46;
    let height = KEYS.len() as u16 + 2;

    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);

    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);

    let lines: Vec<Line> = KEYS
        .iter()
        .map(|(key, what)| {
            Line::from(vec![
                Span::styled(
                    format!("  {key:<9}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(*what),
            ])
        })
        .collect();

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" keys ")),
        area,
    );
}

fn columns(width: u16) -> (usize, usize, usize) {
    // the detail column absorbs whatever is left, so the fixed ones only shrink
    // once the terminal is genuinely narrow.
    if width < 60 {
        (14, 0, 14)
    } else if width < 90 {
        (16, 14, 15)
    } else {
        (20, 22, 16)
    }
}

fn fit(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let count = s.chars().count();

    if count >= width {
        let cut: String = s.chars().take(width.saturating_sub(2)).collect();

        format!("{cut}… ")
    } else {
        format!("{s}{}", " ".repeat(width - count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_short_content_puts_activity_on_the_right() {
        let [devices, activity] = content_areas(Rect::new(4, 2, 120, 30));

        assert_eq!(devices, Rect::new(4, 2, 81, 30));
        assert_eq!(activity, Rect::new(86, 2, ACTIVITY_WIDTH, 30));
    }

    #[test]
    fn narrow_content_stacks_a_short_activity_pane() {
        let [devices, activity] = content_areas(Rect::new(4, 2, 90, 30));

        assert_eq!(devices, Rect::new(4, 2, 90, 25));
        assert_eq!(activity, Rect::new(4, 27, 90, ACTIVITY_HEIGHT));
    }

    fn scan(pending: &[&str]) -> Scan {
        Scan {
            pending: pending.iter().map(|p| p.to_string()).collect(),
            since: std::time::Instant::now(),
        }
    }

    #[test]
    fn a_scan_names_what_it_is_still_waiting_on() {
        let label = scanning_label(&scan(&["local", "mac"]), 80);

        assert!(
            label.contains("local · mac"),
            "a wait that names no host is a wait no one can act on: {label}"
        );
    }

    #[test]
    fn the_pending_hosts_reach_the_screen() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let mut app = App::new(
            std::sync::Arc::new(tokio::sync::Mutex::new(crate::registry::Registry::default())),
            None,
            tx,
        );

        app.scan = Some(scan(&["local", "mac"]));

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 20)).unwrap();

        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        let screen: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        assert!(
            screen.contains("scanning local · mac"),
            "the survey drew no sign of what it was waiting on"
        );
        assert!(
            screen.contains("so far"),
            "a count taken mid-survey drew as a final one"
        );
    }

    #[test]
    fn too_many_hosts_to_name_get_counted_instead() {
        let long = scan(&["mac", "peer-a", "peer-b", "pixel-9", "sample-app"]);

        assert_eq!(scanning_label(&long, 40).trim(), "⠋ scanning 5 sources");
    }

    #[test]
    fn tall_content_stays_stacked() {
        let [devices, activity] = content_areas(Rect::new(4, 2, 120, 50));

        assert_eq!(devices, Rect::new(4, 2, 120, 45));
        assert_eq!(activity, Rect::new(4, 47, 120, ACTIVITY_HEIGHT));
    }
}
