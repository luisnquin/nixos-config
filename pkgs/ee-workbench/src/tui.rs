use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Tabs, Wrap};

use crate::cmd;
use crate::model::{Experiment, Measurement, Project, Stock};
use crate::store::Workbench;

const TABS: [&str; 4] = ["Projects", "Inventory", "Experiments", "Measurements"];

/// Read-only on purpose: the ledger is append-only and every mutation is a
/// reviewable Git change, so writes stay on the subcommands where the
/// arguments are explicit.
struct App {
    workbench: Workbench,
    tab: usize,
    cursor: [usize; TABS.len()],
    projects: Vec<Project>,
    stock: Vec<Stock>,
    experiments: Vec<Experiment>,
    measurements: Vec<Measurement>,
    notice: String,
    quit: bool,
}

impl App {
    fn new(workbench: Workbench) -> Result<Self> {
        let mut app = Self {
            workbench,
            tab: 0,
            cursor: [0; TABS.len()],
            projects: Vec::new(),
            stock: Vec::new(),
            experiments: Vec::new(),
            measurements: Vec::new(),
            notice: String::new(),
            quit: false,
        };

        app.reload()?;

        Ok(app)
    }

    fn reload(&mut self) -> Result<()> {
        self.projects = self.workbench.list_projects()?;
        self.stock = self.workbench.stock()?;
        self.experiments = self.workbench.list_experiments()?;
        self.measurements = self.workbench.list_measurements()?;
        self.measurements.reverse();

        for (tab, len) in self.lengths().into_iter().enumerate() {
            self.cursor[tab] = self.cursor[tab].min(len.saturating_sub(1));
        }

        Ok(())
    }

    fn lengths(&self) -> [usize; TABS.len()] {
        [
            self.projects.len(),
            self.stock.len(),
            self.experiments.len(),
            self.measurements.len(),
        ]
    }

    fn len(&self) -> usize {
        self.lengths()[self.tab]
    }

    fn selected(&self) -> usize {
        self.cursor[self.tab]
    }

    fn move_by(&mut self, delta: isize) {
        let len = self.len();

        if len == 0 {
            return;
        }

        let current = self.selected() as isize;
        self.cursor[self.tab] = current.saturating_add(delta).clamp(0, len as isize - 1) as usize;
    }

    fn on_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.tab = (self.tab + 1) % TABS.len();
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.tab = (self.tab + TABS.len() - 1) % TABS.len();
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Home | KeyCode::Char('g') => self.cursor[self.tab] = 0,
            KeyCode::End | KeyCode::Char('G') => self.move_by(isize::MAX),
            KeyCode::Char('r') => {
                self.reload()?;
                self.notice = "reloaded".into();
            }
            _ => {}
        }

        Ok(())
    }

    fn rows(&self) -> Vec<String> {
        match self.tab {
            0 => self
                .projects
                .iter()
                .map(|project| {
                    format!(
                        "{}  [{}]",
                        project.slug,
                        format!("{:?}", project.status).to_lowercase()
                    )
                })
                .collect(),
            1 => self
                .stock
                .iter()
                .map(|entry| format!("{}  x{}", entry.part, entry.on_hand))
                .collect(),
            2 => self
                .experiments
                .iter()
                .map(|experiment| {
                    format!(
                        "{}  [{}]",
                        experiment.reference(),
                        format!("{:?}", experiment.status).to_lowercase()
                    )
                })
                .collect(),
            _ => self
                .measurements
                .iter()
                .map(|measurement| {
                    format!(
                        "{}  {} {}",
                        measurement.quantity, measurement.value, measurement.unit
                    )
                })
                .collect(),
        }
    }

    fn detail(&self) -> Vec<Line<'static>> {
        let index = self.selected();

        let field = |name: &str, value: String| {
            Line::from(vec![
                Span::styled(format!("{name:<12}"), Style::new().fg(Color::DarkGray)),
                Span::raw(value),
            ])
        };

        match self.tab {
            0 => match self.projects.get(index) {
                Some(project) => {
                    let mut lines = vec![
                        field("project", project.slug.clone()),
                        field("id", project.id.clone()),
                        field("title", project.title.clone()),
                        field("status", format!("{:?}", project.status).to_lowercase()),
                        field("created", cmd::stamp(project.created_at)),
                    ];

                    if let Some(summary) = &project.summary {
                        lines.push(field("summary", summary.clone()));
                    }

                    if !project.tags.is_empty() {
                        lines.push(field("tags", project.tags.join(", ")));
                    }

                    lines.push(field(
                        "experiments",
                        self.experiments
                            .iter()
                            .filter(|experiment| experiment.project == project.slug)
                            .count()
                            .to_string(),
                    ));

                    lines
                }
                None => vec![Line::raw("no projects yet — ee project new <slug>")],
            },
            1 => match self.stock.get(index) {
                Some(entry) => vec![
                    field("part", entry.part.clone()),
                    field("name", entry.name.clone()),
                    field("on hand", entry.on_hand.to_string()),
                    field("events", entry.events.to_string()),
                    field(
                        "last event",
                        entry
                            .last_event_at
                            .map(cmd::stamp)
                            .unwrap_or_else(|| "-".into()),
                    ),
                ],
                None => vec![Line::raw("no parts yet — ee inventory part add <slug>")],
            },
            2 => match self.experiments.get(index) {
                Some(experiment) => {
                    let mut lines = vec![
                        field("experiment", experiment.reference()),
                        field("title", experiment.title.clone()),
                        field("status", format!("{:?}", experiment.status).to_lowercase()),
                        field("updated", cmd::stamp(experiment.updated_at)),
                    ];

                    if let Some(hypothesis) = &experiment.hypothesis {
                        lines.push(field("hypothesis", hypothesis.clone()));
                    }

                    lines.push(field(
                        "measurements",
                        self.measurements
                            .iter()
                            .filter(|measurement| {
                                measurement.project == experiment.project
                                    && measurement.experiment.as_deref()
                                        == Some(experiment.slug.as_str())
                            })
                            .count()
                            .to_string(),
                    ));

                    lines
                }
                None => vec![Line::raw(
                    "no experiments yet — ee experiment new <project>/<slug>",
                )],
            },
            _ => match self.measurements.get(index) {
                Some(measurement) => {
                    let mut lines = vec![
                        field("event", measurement.id.clone()),
                        field("project", measurement.project.clone()),
                        field("quantity", measurement.quantity.clone()),
                        field(
                            "value",
                            format!("{} {}", measurement.value, measurement.unit),
                        ),
                        field("at", cmd::stamp(measurement.at)),
                    ];

                    if let Some(experiment) = &measurement.experiment {
                        lines.push(field("experiment", experiment.clone()));
                    }

                    if let Some(instrument) = &measurement.instrument {
                        lines.push(field("instrument", instrument.clone()));
                    }

                    if let Some(note) = &measurement.note {
                        lines.push(field("note", note.clone()));
                    }

                    lines
                }
                None => vec![Line::raw(
                    "no measurements yet — ee measurement record --project ...",
                )],
            },
        }
    }
}

pub fn run(workbench: Workbench) -> Result<i32> {
    let mut app = App::new(workbench)?;
    let mut terminal = ratatui::init();

    let outcome = loop {
        terminal.draw(|frame| render(frame, &mut app))?;

        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                if let Err(error) = app.on_key(key) {
                    app.notice = format!("{error:#}");
                }
            }
            Ok(_) => {}
            Err(error) => break Err(error),
        }

        if app.quit {
            break Ok(());
        }
    };

    ratatui::restore();

    outcome?;

    Ok(0)
}

fn render(frame: &mut Frame, app: &mut App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let tabs = Tabs::new(TABS.iter().map(|title| Span::raw(*title)))
        .select(app.tab)
        .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan))
        .divider(" ");

    frame.render_widget(tabs, header);

    let [left, right] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(body);

    render_list(frame, app, left);

    let detail = Paragraph::new(app.detail())
        .block(Block::bordered().title(" detail "))
        .wrap(Wrap { trim: false });

    frame.render_widget(detail, right);

    let hint = if app.notice.is_empty() {
        format!(
            "{}  ·  tab/h/l switch · j/k move · r reload · q quit · read-only",
            app.workbench.root.display()
        )
    } else {
        app.notice.clone()
    };

    frame.render_widget(
        Paragraph::new(Span::styled(hint, Style::new().fg(Color::DarkGray))),
        footer,
    );
}

fn render_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let rows = app.rows();
    let title = format!(" {} ({}) ", TABS[app.tab], rows.len());

    let items: Vec<ListItem> = rows.into_iter().map(ListItem::new).collect();

    let list = List::new(items)
        .block(Block::bordered().title(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("");

    let mut state = ListState::default();

    if app.len() > 0 {
        state.select(Some(app.selected()));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
