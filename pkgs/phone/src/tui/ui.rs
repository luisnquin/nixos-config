use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{row_fields, App, Level, Mode};
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
    ("/", "filter"),
    ("g/G", "top/bottom"),
    ("?", "help"),
    ("q", "quit"),
];

pub fn render(frame: &mut Frame, app: &mut App) {
    let [header, list, log, status, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(8),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, app, header);
    render_list(frame, app, list);
    render_log(frame, app, log);
    render_status(frame, app, status);
    render_footer(frame, footer, app);

    if matches!(app.mode, Mode::Help) {
        render_help(frame, frame.area());
    }
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
        spans.push(Span::styled("   default: ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(label, Style::default().fg(Color::Magenta)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::bordered().title(" devices ");
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
                Span::styled(detail, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(rows).block(block).highlight_symbol("▸ ").highlight_style(
        Style::default()
            .bg(Color::Rgb(40, 44, 52))
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, area, &mut app.state);
}

fn reach_style(reach: &Reach) -> Style {
    let color = match reach {
        Reach::Attached { .. } => Color::Green,
        Reach::Online => Color::Cyan,
        Reach::Unauthorized { .. } => Color::Yellow,
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
        (None, Some(queued)) => Line::from(Span::styled(
            format!(" waiting to shoot {queued}"),
            Style::default().fg(Color::Yellow),
        )),
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
