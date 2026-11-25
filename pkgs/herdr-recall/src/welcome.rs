
use std::io;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const GROUP: usize = 3;
const MAX_WIDTH: u16 = 72;

const TEXT: Color = Color::Rgb(0xe0, 0xde, 0xf4);
const SUBTLE: Color = Color::Rgb(0x90, 0x8c, 0xaa);
const MUTED: Color = Color::Rgb(0x6e, 0x6a, 0x86);
const LOVE: Color = Color::Rgb(0xeb, 0x6f, 0x92);
const GROUP_COLORS: [Color; 4] = [
    Color::Rgb(0x9c, 0xcf, 0xd8),
    Color::Rgb(0xc4, 0xa7, 0xe7),
    Color::Rgb(0xeb, 0xbc, 0xba),
    Color::Rgb(0x3e, 0x8f, 0xb0),
];

const LOGO: [&str; 12] = [
    "        ▄▄▄  ▄▄▄▄▄",
    "         ▄ ████████▄",
    "     ▄█████▄ ▄▄▄ ▀██▄ ▄ ▄▄▄    ▄",
    "   ▄██ ▀█████▀▀█▀ ██▀ ███████▄██",
    " ▄████▀▄█▄▄█  ▄▄▄██▀ ▄██████████",
    "▄████████████▄ ▀▀▀ ▄████████████",
    "▀███████████████████████████████",
    "  ▀▀▀▀▀▀████████████████████████",
    "         ███████████████████████",
    "         ▀██████████████████████",
    "          ▀█████████████████████",
    "          ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀",
];

#[derive(Default)]
pub struct Space {
    pub label: String,
    pub icon: String,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub titles: Vec<String>,
    pub panes: Vec<(String, String)>,
}

pub struct Screen {
    title: &'static str,
    subtitle: &'static str,
    enter: &'static str,
    esc: &'static str,
    marks: bool,
}

pub const START: Screen = Screen {
    title: "Welcome back.",
    subtitle: "Enter starts the groups with a thick bar; the rest stay typed.",
    enter: "start the thick-bar groups",
    esc: "leave them all typed",
    marks: true,
};

pub const PARK: Screen = Screen {
    title: "Pause every agent?",
    subtitle: "Enter stops them and leaves their resume line typed.",
    enter: "pause them all",
    esc: "cancel, touch nothing",
    marks: false,
};

pub fn run(spaces: &[Space], screen: &Screen) -> io::Result<Option<Vec<usize>>> {
    let groups = spaces.len().div_ceil(GROUP);
    let mut cursor = 0;
    let mut marked = vec![false; groups];
    let mut terminal = ratatui::init();
    let result = (|| -> io::Result<Option<Vec<usize>>> {
        loop {
            terminal.draw(|frame| draw(frame, spaces, cursor, &starting(&marked, cursor, screen), screen))?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
                KeyCode::Down | KeyCode::Char('j') => cursor = (cursor + 1).min(groups - 1),
                KeyCode::Up | KeyCode::Char('k') => cursor = cursor.saturating_sub(1),
                KeyCode::Char(' ') => marked[cursor] = !marked[cursor],
                KeyCode::Enter => {
                    let picked = starting(&marked, cursor, screen).into_iter();
                    return Ok(Some(picked.flat_map(|group| group * GROUP..((group + 1) * GROUP).min(spaces.len())).collect()));
                }
                _ => continue,
            }
        }
    })();
    ratatui::restore();
    result
}

fn starting(marked: &[bool], cursor: usize, screen: &Screen) -> Vec<usize> {
    if !screen.marks {
        return (0..marked.len()).collect();
    }
    let picked: Vec<usize> = (0..marked.len()).filter(|&group| marked[group]).collect();
    if picked.is_empty() { vec![cursor] } else { picked }
}

fn draw(frame: &mut Frame, spaces: &[Space], cursor: usize, starting: &[usize], screen: &Screen) {
    let full = frame.area();
    let width = full.width.saturating_sub(4).min(MAX_WIDTH);
    let area = Rect {
        x: full.x + (full.width - width) / 2,
        width,
        ..full.inner(Margin::new(0, 1))
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut selected_end = 0;
    for (index, chunk) in spaces.chunks(GROUP).enumerate() {
        let color = GROUP_COLORS[index % GROUP_COLORS.len()];
        let selected = index == cursor;
        let bar = Span::raw(if starting.contains(&index) { "▌ " } else { "│ " }).fg(color);
        for space in chunk {
            space_lines(&mut lines, space, &bar, color, selected, width);
        }
        if selected {
            selected_end = lines.len();
        }
        lines.push(Line::default());
    }
    lines.pop();

    let show_logo = area.height as usize >= LOGO.len() + 1 + 3 + lines.len() + 4;
    let [logo, question, list, footer] = Layout::vertical([
        Constraint::Length(if show_logo { LOGO.len() as u16 + 1 } else { 0 }),
        Constraint::Length(3),
        Constraint::Max(lines.len() as u16),
        Constraint::Length(4),
    ])
    .flex(Flex::Center)
    .areas(area);

    frame.render_widget(
        Paragraph::new(LOGO.iter().map(|row| Line::from(*row).fg(SUBTLE)).collect::<Vec<_>>()),
        logo,
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(screen.title).fg(TEXT).bold(),
            Line::from(screen.subtitle).fg(SUBTLE),
        ]),
        question,
    );
    let scroll = selected_end.saturating_sub(list.height as usize) as u16;
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), list);

    let key = |key: &'static str, what: &'static str| {
        vec![Span::raw(format!("{key:<6}")).fg(TEXT).bold(), Span::raw(format!("{what:<30}")).fg(SUBTLE)]
    };
    let footer_lines = vec![
        Line::default(),
        Line::from([key("enter", screen.enter), key("↑↓", "move")].concat()),
        if screen.marks {
            Line::from([key("space", "mark or unmark a group"), key("esc", screen.esc)].concat())
        } else {
            Line::from(key("esc", screen.esc))
        },
    ];
    frame.render_widget(Paragraph::new(footer_lines), footer);
}

fn space_lines(lines: &mut Vec<Line>, space: &Space, bar: &Span<'static>, color: Color, selected: bool, width: u16) {
    lines.push(Line::from(vec![
        bar.clone(),
        Span::raw("· ").fg(MUTED),
        Span::raw(format!("{} ", space.icon)).fg(color),
        Span::raw("· ").fg(MUTED),
        Span::raw(space.label.clone()).fg(if selected { TEXT } else { SUBTLE }).bold(),
    ]));
    let mut git = vec![bar.clone(), Span::raw(format!("  {}", space.branch)).fg(MUTED)];
    if space.ahead > 0 {
        git.push(Span::raw(format!(" ↑{}", space.ahead)).fg(color));
    }
    if space.behind > 0 {
        git.push(Span::raw(format!(" ↓{}", space.behind)).fg(LOVE));
    }
    if !space.branch.is_empty() {
        lines.push(Line::from(git));
    }
    for title in &space.titles {
        let room = (width as usize).saturating_sub(8);
        lines.push(Line::from(vec![
            bar.clone(),
            Span::raw(format!("    {}", crate::truncate(title, room))).fg(if selected { SUBTLE } else { MUTED }),
        ]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_starts_marked_groups_else_the_cursor() {
        assert_eq!(starting(&[false, false, false], 1, &START), vec![1]);
        assert_eq!(starting(&[true, false, true], 1, &START), vec![0, 2]);
        assert_eq!(starting(&[false, false], 1, &PARK), vec![0, 1]);
    }

    #[test]
    fn logo_fits_its_width() {
        assert!(LOGO.iter().all(|row| row.chars().count() <= 32));
    }
}
