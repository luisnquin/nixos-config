use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Row, Table, TableState},
    DefaultTerminal, Frame,
};

const REFRESH: Duration = Duration::from_secs(2);
const FETCH_EVERY: Duration = Duration::from_secs(300);

type GitOp = (&'static str, &'static [&'static str]);
const PUSH: GitOp = ("push", &["push"]);
const PULL: GitOp = ("pull", &["pull", "--ff-only"]);
const DIM: Style = Style::new().fg(Color::DarkGray);

#[derive(Clone, Default)]
struct Status {
    branch: String,
    added: u64,
    removed: u64,
    ahead: Option<u64>,
    behind: Option<u64>,
}

enum Msg {
    Statuses(Vec<Status>),
    Note(usize, String),
    Fetching,
    Fetched,
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = stderr.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let reason = lines
        .iter()
        .find(|l| l.starts_with('!') || l.starts_with("fatal") || l.starts_with("error"))
        .or(lines.last())
        .copied()
        .unwrap_or("failed");
    Err(reason.to_owned())
}

fn outermost_repo(start: &Path) -> Result<PathBuf> {
    let mut top = git(start, &["rev-parse", "--show-toplevel"])
        .map_err(anyhow::Error::msg)
        .context("not inside a git repository")?;
    while let Ok(parent) = git(Path::new(&top), &["rev-parse", "--show-superproject-working-tree"]) {
        if parent.is_empty() {
            break;
        }
        top = parent;
    }
    Ok(top.into())
}

fn repos(root: &Path) -> Vec<(String, PathBuf)> {
    let name = root
        .file_name()
        .map_or_else(|| ".".to_owned(), |n| n.to_string_lossy().into_owned());
    let mut out = vec![(name, root.to_path_buf())];
    let foreach = ["submodule", "foreach", "--recursive", "--quiet", "echo \"$displaypath\""];
    if let Ok(list) = git(root, &foreach) {
        out.extend(list.lines().map(|p| (p.to_owned(), root.join(p))));
    }
    out
}

fn parse_shortstat(s: &str) -> (u64, u64) {
    let (mut added, mut removed) = (0, 0);
    for part in s.split(',').map(str::trim) {
        let n = part.split(' ').next().and_then(|n| n.parse().ok()).unwrap_or(0);
        if part.contains("insertion") {
            added = n;
        } else if part.contains("deletion") {
            removed = n;
        }
    }
    (added, removed)
}

fn status(dir: &Path) -> Status {
    let branch = git(dir, &["branch", "--show-current"])
        .ok()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "detached".to_owned());
    let (added, removed) = git(dir, &["diff", "--shortstat", "HEAD"])
        .map(|s| parse_shortstat(&s))
        .unwrap_or_default();
    let counts = git(dir, &["rev-list", "--left-right", "--count", "HEAD...@{u}"]).unwrap_or_default();
    let mut counts = counts.split_whitespace().map(|n| n.parse().ok());
    Status {
        branch,
        added,
        removed,
        ahead: counts.next().flatten(),
        behind: counts.next().flatten(),
    }
}

fn fetch_all(paths: &[PathBuf], tx: &Sender<Msg>) {
    let _ = tx.send(Msg::Fetching);
    thread::scope(|s| {
        for (i, dir) in paths.iter().enumerate() {
            s.spawn(move || {
                if let Err(e) = git(dir, &["fetch", "--quiet"]) {
                    let _ = tx.send(Msg::Note(i, format!("fetch failed: {e}")));
                }
            });
        }
    });
    let _ = tx.send(Msg::Fetched);
}

struct App {
    root: PathBuf,
    names: Vec<String>,
    paths: Vec<PathBuf>,
    statuses: Vec<Status>,
    notes: Vec<String>,
    marked: Vec<bool>,
    state: TableState,
    fetching: bool,
    last_fetch: Option<Instant>,
    tx: Sender<Msg>,
}

impl App {
    fn run_git(&mut self, targets: Vec<usize>, (verb, args): GitOp) {
        for i in targets {
            self.notes[i] = format!("{verb}…");
            let (tx, dir) = (self.tx.clone(), self.paths[i].clone());
            thread::spawn(move || {
                let note = match git(&dir, args) {
                    Ok(_) => format!("{verb} ok"),
                    Err(e) => format!("{verb} failed: {e}"),
                };
                let _ = tx.send(Msg::Note(i, note));
            });
        }
    }

    fn take_targets(&mut self, cursor: usize) -> Vec<usize> {
        let marked: Vec<usize> = (0..self.marked.len()).filter(|&i| self.marked[i]).collect();
        self.marked.fill(false);
        if marked.is_empty() { vec![cursor] } else { marked }
    }

    fn where_status(&self, pick: fn(&Status) -> Option<u64>) -> Vec<usize> {
        (0..self.statuses.len()).filter(|&i| pick(&self.statuses[i]).unwrap_or(0) > 0).collect()
    }

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::Statuses(s) => self.statuses = s,
            Msg::Note(i, n) => self.notes[i] = n,
            Msg::Fetching => self.fetching = true,
            Msg::Fetched => {
                self.fetching = false;
                self.last_fetch = Some(Instant::now());
            }
        }
    }

    fn on_key(&mut self, code: KeyCode, terminal: &mut DefaultTerminal) -> bool {
        let last = self.paths.len() - 1;
        let i = self.state.selected().unwrap_or(0).min(last);
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('j') | KeyCode::Down => self.state.select(Some((i + 1).min(last))),
            KeyCode::Char('k') | KeyCode::Up => self.state.select(Some(i.saturating_sub(1))),
            KeyCode::Char(' ') => {
                self.marked[i] = !self.marked[i];
                self.state.select(Some((i + 1).min(last)));
            }
            KeyCode::Char('p') => {
                let targets = self.take_targets(i);
                self.run_git(targets, PUSH);
            }
            KeyCode::Char('l') => {
                let targets = self.take_targets(i);
                self.run_git(targets, PULL);
            }
            KeyCode::Char('P') => self.run_git(self.where_status(|s| s.ahead), PUSH),
            KeyCode::Char('L') => self.run_git(self.where_status(|s| s.behind), PULL),
            KeyCode::Char('f') if !self.fetching => {
                let (tx, paths) = (self.tx.clone(), self.paths.clone());
                thread::spawn(move || fetch_all(&paths, &tx));
            }
            KeyCode::Enter => {
                ratatui::restore();
                let _ = Command::new("lazygit").arg("-p").arg(&self.paths[i]).status();
                *terminal = ratatui::init();
            }
            _ => {}
        }
        false
    }

    fn run(mut self, terminal: &mut DefaultTerminal, rx: Receiver<Msg>) -> Result<()> {
        loop {
            while let Ok(msg) = rx.try_recv() {
                self.apply(msg);
            }
            terminal.draw(|f| self.draw(f))?;
            if !event::poll(Duration::from_millis(200))? {
                continue;
            }
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && self.on_key(key.code, terminal) {
                    return Ok(());
                }
            }
        }
    }

    fn row(&self, i: usize) -> Row<'static> {
        let s = self.statuses.get(i).cloned().unwrap_or_default();
        let mut diff = Vec::new();
        if s.added > 0 {
            diff.push(Span::raw(format!("+{} ", s.added)).green());
        }
        if s.removed > 0 {
            diff.push(Span::raw(format!("-{}", s.removed)).red());
        }
        let sync = match (s.ahead, s.behind) {
            (None, _) | (_, None) => Line::styled("no upstream", DIM),
            (Some(0), Some(0)) => Line::styled("=", DIM),
            (Some(a), Some(b)) => Line::from(vec![
                if a > 0 { Span::raw(format!("↑{a} ")).yellow() } else { Span::raw("") },
                if b > 0 { Span::raw(format!("↓{b}")).cyan() } else { Span::raw("") },
            ]),
        };
        let note = &self.notes[i];
        let note_style = if note.contains("failed") { Style::new().red() } else { DIM };
        Row::new(vec![
            Line::raw(if self.marked[i] { "●" } else { " " }),
            Line::raw(self.names[i].clone()),
            Line::styled(s.branch, DIM),
            Line::from(diff),
            sync,
            Line::styled(note.clone(), note_style),
        ])
    }

    fn draw(&mut self, f: &mut Frame) {
        let [head, body, foot] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(f.area());

        let fetch = match (self.fetching, self.last_fetch) {
            (true, _) => "fetching…".to_owned(),
            (false, Some(t)) => format!("fetched {}m ago", t.elapsed().as_secs() / 60),
            (false, None) => "not fetched".to_owned(),
        };
        f.render_widget(
            Line::from(vec![
                Span::raw(self.root.display().to_string()).bold(),
                Span::raw(format!("  {fetch}")).dark_gray(),
            ]),
            head,
        );

        let rows = (0..self.names.len()).map(|i| self.row(i));
        let name_w = self.names.iter().map(|n| n.chars().count()).max().unwrap_or(0) as u16;
        let table = Table::new(
            rows,
            [
                Constraint::Length(1),
                Constraint::Length(name_w),
                Constraint::Length(12),
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Fill(1),
            ],
        )
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(table, body, &mut self.state);

        f.render_widget(
            Line::styled(
                "space mark  p/l push/pull marked or cursor  P/L push all ahead/pull all behind  f fetch  enter lazygit  q quit",
                DIM,
            ),
            foot,
        );
    }
}

fn main() -> Result<()> {
    let start = env::args_os().nth(1).map_or_else(env::current_dir, |p| Ok(p.into()))?;
    let root = outermost_repo(&start)?;
    let (names, paths): (Vec<String>, Vec<PathBuf>) = repos(&root).into_iter().unzip();
    let (tx, rx) = mpsc::channel();

    let (status_tx, status_paths) = (tx.clone(), paths.clone());
    thread::spawn(move || {
        while status_tx.send(Msg::Statuses(status_paths.iter().map(|p| status(p)).collect())).is_ok() {
            thread::sleep(REFRESH);
        }
    });
    let (fetch_tx, fetch_paths) = (tx.clone(), paths.clone());
    thread::spawn(move || loop {
        fetch_all(&fetch_paths, &fetch_tx);
        thread::sleep(FETCH_EVERY);
    });

    let n = paths.len();
    let app = App {
        root,
        names,
        paths,
        statuses: Vec::new(),
        notes: vec![String::new(); n],
        marked: vec![false; n],
        state: TableState::default().with_selected(0),
        fetching: false,
        last_fetch: None,
        tx,
    };
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal, rx);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::parse_shortstat;

    #[test]
    fn shortstat() {
        assert_eq!(parse_shortstat(" 3 files changed, 34 insertions(+), 6 deletions(-)"), (34, 6));
        assert_eq!(parse_shortstat(" 1 file changed, 1 insertion(+)"), (1, 0));
        assert_eq!(parse_shortstat(" 1 file changed, 2 deletions(-)"), (0, 2));
        assert_eq!(parse_shortstat(""), (0, 0));
    }
}
