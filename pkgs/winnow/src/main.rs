use std::fs::File;
use std::io::{self, BufRead, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::{cursor, execute, terminal};
use ratatui::layout::{Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

const USAGE: &str = "usage: winnow [--prompt STR] [--header STR] [--height N|N%] [--delimiter STR]
              [--with-nth N] [--print-query] [--expect KEY[,KEY..]] [--cursor N]
              [--multi] [--checked N,N..] [--ansi]
              [--border[=STYLE]] [--pointer STR] [--separator STR] [--scrollbar STR] [--color SPEC]

Reads lines from stdin, prints the chosen one. Enter accepts, Esc aborts (exit 130).
--multi: space toggles, Enter prints every checked line, or the current one when none is checked;
         --expect keys print only the checked lines.
--checked pre-checks input lines by 1-based number.
--ansi renders SGR colors in the shown field.
--expect keys accept too and print the key name first (empty line for Enter).
Keys: del, tab, btab, left, right, alt-enter, alt-X, ctrl-X.
Style flags in FZF_DEFAULT_OPTS apply first; flags winnow does not know are ignored there.";

struct Theme {
    fg: Style,
    bg: Style,
    fg_sel: Style,
    bg_sel: Style,
    hl: Style,
    hl_sel: Style,
    header: Style,
    info: Style,
    pointer: Style,
    prompt: Style,
    query: Style,
    border: Style,
    separator: Option<Style>,
    scrollbar: Option<Style>,
}

impl Default for Theme {
    fn default() -> Self {
        let fg = |c| Style::default().fg(c);
        Self {
            fg: Style::default(),
            bg: Style::default(),
            fg_sel: Style::default().add_modifier(Modifier::BOLD),
            bg_sel: Style::default().bg(Color::Indexed(236)),
            hl: fg(Color::Green),
            hl_sel: fg(Color::Green),
            header: fg(Color::Cyan),
            info: fg(Color::Yellow),
            pointer: fg(Color::Magenta),
            prompt: fg(Color::Blue),
            query: Style::default(),
            border: fg(Color::DarkGray),
            separator: None,
            scrollbar: None,
        }
    }
}

impl Theme {
    fn slot(&mut self, key: &str) -> Option<&mut Style> {
        Some(match key {
            "fg" => &mut self.fg,
            "bg" => &mut self.bg,
            "fg+" => &mut self.fg_sel,
            "bg+" => &mut self.bg_sel,
            "hl" => &mut self.hl,
            "hl+" => &mut self.hl_sel,
            "header" => &mut self.header,
            "info" => &mut self.info,
            "pointer" => &mut self.pointer,
            "prompt" => &mut self.prompt,
            "query" => &mut self.query,
            "border" => &mut self.border,
            "separator" => self.separator.get_or_insert_with(Style::default),
            "scrollbar" => self.scrollbar.get_or_insert_with(Style::default),
            _ => return None,
        })
    }

    fn apply(&mut self, spec: &str) {
        for entry in spec.split(',') {
            let mut parts = entry.split(':');
            let key = parts.next().unwrap_or_default();
            let Some(slot) = self.slot(key) else { continue };
            for value in parts {
                *slot = restyle(*slot, key.starts_with("bg"), value);
            }
        }
    }
}

fn restyle(style: Style, background: bool, value: &str) -> Style {
    let modifier = match value {
        "bold" => Modifier::BOLD,
        "dim" => Modifier::DIM,
        "italic" => Modifier::ITALIC,
        "underline" => Modifier::UNDERLINED,
        "reverse" => Modifier::REVERSED,
        _ => Modifier::empty(),
    };
    let color = if value == "-1" { Ok(Color::Reset) } else { Color::from_str(value) };
    match color {
        Ok(c) if background => style.bg(c),
        Ok(c) => style.fg(c),
        Err(_) => style.add_modifier(modifier),
    }
}

struct Opts {
    prompt: String,
    header: Vec<String>,
    height: (u16, bool),
    delimiter: Option<String>,
    nth: Option<usize>,
    print_query: bool,
    multi: bool,
    checked: Vec<usize>,
    ansi: bool,
    expect: Vec<(String, KeyCode, KeyModifiers)>,
    cursor: usize,
    border: Option<BorderType>,
    pointer: String,
    separator: String,
    scrollbar: String,
    theme: Theme,
}

fn parse_key(name: &str) -> Result<(KeyCode, KeyModifiers)> {
    let single = |s: &str| {
        let mut c = s.chars();
        match (c.next(), c.next()) {
            (Some(ch), None) => Some(ch),
            _ => None,
        }
    };
    let named = match name {
        "del" | "delete" => Some((KeyCode::Delete, KeyModifiers::NONE)),
        "tab" => Some((KeyCode::Tab, KeyModifiers::NONE)),
        "btab" => Some((KeyCode::BackTab, KeyModifiers::NONE)),
        "left" => Some((KeyCode::Left, KeyModifiers::NONE)),
        "right" => Some((KeyCode::Right, KeyModifiers::NONE)),
        "alt-enter" => Some((KeyCode::Enter, KeyModifiers::ALT)),
        _ => None,
    };
    let chord = name
        .strip_prefix("alt-")
        .and_then(|r| single(r).map(|c| (KeyCode::Char(c), KeyModifiers::ALT)))
        .or_else(|| name.strip_prefix("ctrl-").and_then(|r| single(r).map(|c| (KeyCode::Char(c), KeyModifiers::CONTROL))));
    named.or(chord).with_context(|| format!("unknown key: {name}"))
}

fn shell_words(s: &str) -> Vec<String> {
    let mut words = vec![];
    let mut cur = String::new();
    let mut in_word = false;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => cur.extend(chars.by_ref().take_while(|&c| c != '\'')),
            '"' => {
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => cur.extend(chars.next()),
                        _ => cur.push(c),
                    }
                }
            }
            '\\' => cur.extend(chars.next()),
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
                continue;
            }
            c => cur.push(c),
        }
        in_word = true;
    }
    if in_word {
        words.push(cur);
    }
    words
}

const VALUE_FLAGS: [&str; 12] = [
    "--prompt", "--checked", "--header", "--height", "--delimiter", "--with-nth", "--expect", "--cursor", "--pointer", "--separator", "--scrollbar",
    "--color",
];

fn parse_height(v: &str) -> Result<(u16, bool)> {
    Ok((v.trim_start_matches('~').trim_end_matches('%').parse().context("--height")?, v.ends_with('%')))
}

fn parse_border(style: Option<&str>) -> BorderType {
    match style.unwrap_or("rounded") {
        "sharp" => BorderType::Plain,
        "bold" => BorderType::Thick,
        "double" => BorderType::Double,
        _ => BorderType::Rounded,
    }
}

fn parse_expect(v: &str) -> Result<Vec<(String, KeyCode, KeyModifiers)>> {
    v.split(',')
        .filter(|s| !s.is_empty())
        .map(|name| parse_key(name).map(|(code, mods)| (name.to_string(), code, mods)))
        .collect()
}

fn apply_switch(o: &mut Opts, flag: &str, inline: Option<&str>) -> bool {
    match flag {
        "--border" => o.border = (inline != Some("none")).then(|| parse_border(inline)),
        "--no-border" => o.border = None,
        "--print-query" => o.print_query = true,
        "--ansi" => o.ansi = true,
        "--multi" => o.multi = true,
        "--no-separator" => o.separator.clear(),
        "--no-scrollbar" => o.scrollbar.clear(),
        _ => return false,
    }
    true
}

fn apply_style_value(o: &mut Opts, flag: &str, v: String) {
    match flag {
        "--pointer" => o.pointer = v,
        "--separator" => o.separator = v,
        "--scrollbar" => o.scrollbar = v,
        "--color" => o.theme.apply(&v),
        _ => unreachable!("VALUE_FLAGS lists only flags handled above"),
    }
}

fn apply_value(o: &mut Opts, flag: &str, v: String) -> Result<()> {
    match flag {
        "--prompt" => o.prompt = v,
        "--header" => o.header = v.lines().map(String::from).collect(),
        "--height" => o.height = parse_height(&v)?,
        "--delimiter" => o.delimiter = Some(v),
        "--with-nth" => o.nth = Some(v.parse().context("--with-nth")?),
        "--expect" => o.expect.extend(parse_expect(&v)?),
        "--cursor" => o.cursor = v.parse().context("--cursor")?,
        "--checked" => {
            o.checked = v.split(',').filter(|s| !s.is_empty()).map(str::parse).collect::<Result<_, _>>().context("--checked")?
        }
        _ => apply_style_value(o, flag, v),
    }
    Ok(())
}

fn apply_args(o: &mut Opts, args: Vec<String>, strict: bool) -> Result<()> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        if apply_switch(o, flag, inline.as_deref()) {
            continue;
        }
        if VALUE_FLAGS.contains(&flag) {
            let v = inline.or_else(|| args.next()).with_context(|| format!("{flag} needs a value"))?;
            apply_value(o, flag, v)?;
        } else if strict && matches!(flag, "-h" | "--help") {
            println!("{USAGE}");
            std::process::exit(0);
        } else if strict {
            bail!("unknown argument: {arg}\n{USAGE}");
        }
    }
    Ok(())
}

fn parse_args() -> Result<Opts> {
    let mut o = Opts {
        prompt: "> ".into(),
        header: vec![],
        height: (40, true),
        delimiter: None,
        nth: None,
        print_query: false,
        multi: false,
        ansi: false,
        checked: vec![],
        expect: vec![],
        cursor: 1,
        border: None,
        pointer: "▌".into(),
        separator: "─".into(),
        scrollbar: "│".into(),
        theme: Theme::default(),
    };
    if let Ok(env) = std::env::var("FZF_DEFAULT_OPTS") {
        apply_args(&mut o, shell_words(&env), false)?;
    }
    apply_args(&mut o, std::env::args().skip(1).collect(), true)?;
    Ok(o)
}

fn field<'a>(line: &'a str, o: &Opts) -> &'a str {
    match (o.nth, &o.delimiter) {
        (None, _) => line,
        (Some(n), Some(d)) => line.split(d.as_str()).nth(n.saturating_sub(1)).unwrap_or(""),
        (Some(n), None) => line.split_whitespace().nth(n.saturating_sub(1)).unwrap_or(""),
    }
}

fn fuzzy(text: &str, query: &str) -> Option<(i64, Vec<usize>)> {
    let sensitive = query.chars().any(char::is_uppercase);
    let norm = |c: char| if sensitive { c } else { c.to_ascii_lowercase() };
    let chars: Vec<char> = text.chars().collect();
    let mut pos = Vec::new();
    let mut score = 0i64;
    let mut i = 0;
    for q in query.chars().map(norm) {
        while i < chars.len() && norm(chars[i]) != q {
            i += 1;
        }
        if i == chars.len() {
            return None;
        }
        score += 16;
        if pos.last().is_some_and(|&p| p + 1 == i) {
            score += 8;
        }
        if i == 0 || matches!(chars[i - 1], '/' | '_' | '-' | '.' | ' ' | '\t') {
            score += 12;
        }
        pos.push(i);
        i += 1;
    }
    score -= pos.first().copied().unwrap_or(0) as i64;
    Some((score, pos))
}

struct State {
    items: Vec<(String, String)>,
    styles: Vec<Vec<Style>>,
    query: String,
    matches: Vec<(usize, Vec<usize>)>,
    checked: Vec<bool>,
    selected: usize,
    offset: usize,
    page: usize,
}

impl State {
    fn refilter(&mut self) {
        let mut scored: Vec<(i64, usize, Vec<usize>)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, (_, text))| fuzzy(text, &self.query).map(|(s, p)| (s, i, p)))
            .collect();
        if !self.query.is_empty() {
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        self.matches = scored.into_iter().map(|(_, i, p)| (i, p)).collect();
        self.selected = 0;
        self.offset = 0;
    }

    fn current(&self) -> Option<&str> {
        self.matches.get(self.selected).map(|(i, _)| self.items[*i].0.as_str())
    }

    fn toggle(&mut self) {
        let Some(&(idx, _)) = self.matches.get(self.selected) else { return };
        self.checked[idx] ^= true;
        if !self.query.is_empty() {
            self.query.clear();
            self.refilter();
            self.selected = idx;
        }
    }

    fn move_by(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let max = self.matches.len() as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, max) as usize;
    }
}

enum Outcome {
    Accept(Option<String>),
    Abort,
}

fn frame(f: &mut Frame, o: &Opts) -> (Rect, Rect) {
    let t = &o.theme;
    let area = f.area();
    let Some(kind) = o.border else { return (area, area) };
    let block = Block::bordered().border_type(kind).border_style(t.border).style(t.fg.patch(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);
    (inner, inner.inner(Margin::new(1, 0)))
}

fn item_line<'a>(st: &'a State, o: &Opts, row: usize, idx: usize, pos: &[usize]) -> Line<'a> {
    let t = &o.theme;
    let sel = row == st.selected;
    let (text, hl) = if sel { (t.fg_sel, t.fg_sel.patch(t.hl_sel)) } else { (t.fg, t.fg.patch(t.hl)) };
    let marker = if sel {
        Span::styled(format!("{} ", o.pointer), t.pointer)
    } else {
        Span::raw(" ".repeat(o.pointer.chars().count() + 1))
    };
    let mut spans = vec![marker];
    if o.multi {
        spans.push(if st.checked[idx] { Span::styled("● ", t.pointer) } else { Span::styled("○ ", text) });
    }
    spans.extend(
        st.items[idx].1.chars().enumerate().map(|(ci, ch)| {
            let base = st.styles.get(idx).and_then(|s| s.get(ci)).map_or(text, |s| text.patch(*s));
            Span::styled(ch.to_string(), if pos.contains(&ci) { hl } else { base })
        }),
    );
    let line = Line::from(spans);
    if sel { line.style(t.bg_sel) } else { line }
}

fn render_scrollbar(f: &mut Frame, st: &State, o: &Opts, x: u16, top: u16, rows: usize) {
    let total = st.matches.len();
    if o.scrollbar.is_empty() || rows == 0 || total <= rows {
        return;
    }
    let size = (rows * rows / total).max(1);
    let start = st.offset * (rows - size) / (total - rows);
    let style = o.theme.scrollbar.unwrap_or(o.theme.border);
    for y in start..start + size {
        f.buffer_mut()[Position::new(x, top + y as u16)].set_symbol(&o.scrollbar).set_style(style);
    }
}

fn draw(f: &mut Frame, st: &mut State, o: &Opts) {
    let t = &o.theme;
    let (inner, content) = frame(f, o);
    let fixed = 2 + o.header.len();
    let rows = (content.height as usize).saturating_sub(fixed);
    st.page = rows.max(1);
    if st.selected < st.offset {
        st.offset = st.selected;
    } else if rows > 0 && st.selected >= st.offset + rows {
        st.offset = st.selected + 1 - rows;
    }

    let picked = st.checked.iter().filter(|&&c| c).count();
    let info = match (o.multi, picked) {
        (true, 1..) => format!("  {}/{} · {picked} selected ", st.matches.len(), st.items.len()),
        _ => format!("  {}/{} ", st.matches.len(), st.items.len()),
    };
    let sep = o.separator.repeat((content.width as usize).saturating_sub(info.chars().count()));
    let mut lines = vec![
        Line::from(vec![Span::styled(o.prompt.as_str(), t.prompt), Span::styled(st.query.as_str(), t.query)]),
        Line::from(vec![Span::styled(info, t.info), Span::styled(sep, t.separator.unwrap_or(t.border))]),
    ];
    lines.extend(o.header.iter().map(|h| Line::styled(format!("  {h}"), t.header)));
    lines.extend(st.matches.iter().enumerate().skip(st.offset).take(rows).map(|(row, (idx, pos))| item_line(st, o, row, *idx, pos)));
    f.render_widget(Paragraph::new(lines).style(t.fg.patch(t.bg)), content);
    render_scrollbar(f, st, o, inner.right().saturating_sub(1), content.y + fixed as u16, rows);

    let x = content.x + (o.prompt.chars().count() + st.query.chars().count()) as u16;
    f.set_cursor_position(Position::new(x.min(content.right().saturating_sub(1)), content.y));
}

fn edit_query(st: &mut State, key: &KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Backspace => {
            st.query.pop();
        }
        KeyCode::Char('u') if ctrl => st.query.clear(),
        KeyCode::Char('w') if ctrl => {
            let trimmed = st.query.trim_end().len();
            st.query.truncate(st.query[..trimmed].rfind(' ').map_or(0, |i| i + 1));
        }
        KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => st.query.push(c),
        _ => return,
    }
    st.refilter();
}

fn handle_key(st: &mut State, key: &KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let page = st.page as isize;
    match key.code {
        KeyCode::Up => st.move_by(-1),
        KeyCode::Char('p' | 'k') if ctrl => st.move_by(-1),
        KeyCode::Down => st.move_by(1),
        KeyCode::Char('n' | 'j') if ctrl => st.move_by(1),
        KeyCode::PageUp => st.move_by(-page),
        KeyCode::PageDown => st.move_by(page),
        _ => edit_query(st, key),
    }
}

fn run(term: &mut Terminal<CrosstermBackend<File>>, st: &mut State, o: &Opts) -> Result<Outcome> {
    loop {
        term.draw(|f| draw(f, st, o))?;

        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if let Some((name, ..)) = o.expect.iter().find(|(_, c, m)| key_is(&key, *c, *m)) {
            return Ok(Outcome::Accept(Some(name.clone())));
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Ok(Outcome::Abort),
            KeyCode::Char('c' | 'g' | 'q') if ctrl => return Ok(Outcome::Abort),
            KeyCode::Enter => return Ok(Outcome::Accept(o.expect.first().map(|_| String::new()))),
            KeyCode::Char(' ') if o.multi && key.modifiers.is_empty() => st.toggle(),
            _ => handle_key(st, &key),
        }
    }
}

fn key_is(key: &KeyEvent, code: KeyCode, mods: KeyModifiers) -> bool {
    let held = key.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT);
    key.code == code && (code == KeyCode::BackTab || held == mods)
}

struct TtyGuard {
    saved_out: i32,
}

impl TtyGuard {
    fn new(tty: &File) -> Result<Self> {
        let fd = tty.as_raw_fd();
        // SAFETY: fds 0, 1 and `fd` are open for the whole call; dup/dup2 only
        // duplicate descriptors and touch no Rust-owned memory.
        let saved_out = unsafe { libc::dup(1) };
        // SAFETY: same invariants as above; `fd` is borrowed from a live File.
        if saved_out < 0 || unsafe { libc::dup2(fd, 0) } < 0 || unsafe { libc::dup2(fd, 1) } < 0 {
            bail!("cannot attach to /dev/tty: {}", io::Error::last_os_error());
        }
        terminal::enable_raw_mode()?;
        Ok(Self { saved_out })
    }
}

impl Drop for TtyGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        // SAFETY: `saved_out` came from dup() in `new` and is closed only here,
        // once, because Drop runs once.
        unsafe {
            libc::dup2(self.saved_out, 1);
            libc::close(self.saved_out);
        }
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("winnow: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn sgr(mut st: Style, params: &str) -> Style {
    let mut it = params.split(';').map(|p| p.parse::<u8>().unwrap_or(0));
    while let Some(n) = it.next() {
        st = match n {
            0 => Style::default(),
            1 => st.add_modifier(Modifier::BOLD),
            30..=37 => st.fg(Color::Indexed(n - 30)),
            39 => st.fg(Color::Reset),
            90..=97 => st.fg(Color::Indexed(n - 82)),
            38 => match (it.next(), it.next()) {
                (Some(5), Some(i)) => st.fg(Color::Indexed(i)),
                (Some(2), Some(r)) => st.fg(Color::Rgb(r, it.next().unwrap_or(0), it.next().unwrap_or(0))),
                _ => st,
            },
            _ => st,
        };
    }
    st
}

fn strip_ansi(s: &str) -> (String, Vec<Style>) {
    let (mut text, mut styles, mut cur) = (String::new(), vec![], Style::default());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next_if_eq(&'[').is_some() {
            let params: String = chars.by_ref().take_while(|c| !c.is_ascii_alphabetic()).collect();
            cur = sgr(cur, &params);
            continue;
        }
        text.push(c);
        styles.push(cur);
    }
    (text, styles)
}

type Items = (Vec<(String, String)>, Vec<Vec<Style>>);

fn read_items(o: &Opts) -> Result<Items> {
    let lines = io::stdin().lock().lines().collect::<io::Result<Vec<_>>>()?;
    Ok(lines
        .into_iter()
        .map(|line| {
            let (text, styles) = if o.ansi { strip_ansi(field(&line, o)) } else { (field(&line, o).to_string(), vec![]) };
            ((line, text), styles)
        })
        .unzip())
}

fn viewport_height(o: &Opts, rows: u16) -> u16 {
    let wanted = match o.height {
        (n, true) => (rows as u32 * n as u32 / 100).max(10) as u16,
        (n, false) => n,
    };
    wanted.min(rows.saturating_sub(1)).max(3)
}

fn pick(tty: &File, st: &mut State, o: &Opts) -> Result<Outcome> {
    let _guard = TtyGuard::new(tty)?;
    let (col, _) = cursor::position()?;
    let (_, rows) = terminal::size()?;
    write!(tty.try_clone()?, "\r\n")?;
    let mut term = Terminal::with_options(
        CrosstermBackend::new(tty.try_clone()?),
        TerminalOptions { viewport: Viewport::Inline(viewport_height(o, rows)) },
    )?;
    let result = run(&mut term, st, o);
    let top = term.get_frame().area().y;
    term.clear()?;
    execute!(term.backend_mut(), cursor::MoveTo(col, top.saturating_sub(1)), cursor::Show)?;
    result
}

fn print_result(st: &State, o: &Opts, key: Option<String>) -> Result<u8> {
    let mut out = io::stdout().lock();
    if o.print_query {
        writeln!(out, "{}", st.query)?;
    }
    if let Some(key) = &key {
        writeln!(out, "{key}")?;
    }
    if o.multi && (st.checked.contains(&true) || key.as_deref().is_some_and(|k| !k.is_empty())) {
        for (line, _) in st.items.iter().zip(&st.checked).filter(|(_, &c)| c) {
            writeln!(out, "{}", line.0)?;
        }
        return Ok(0);
    }
    let current = st.current();
    if let Some(line) = current {
        writeln!(out, "{line}")?;
    }
    Ok(u8::from(current.is_none()))
}

fn real_main() -> Result<u8> {
    let o = parse_args()?;
    let (items, styles) = read_items(&o)?;
    let mut checked = vec![false; items.len()];
    for n in &o.checked {
        if let Some(c) = n.checked_sub(1).and_then(|i| checked.get_mut(i)) {
            *c = true;
        }
    }
    let mut st = State { items, styles, query: String::new(), matches: vec![], checked, selected: 0, offset: 0, page: 1 };
    st.refilter();
    st.move_by(o.cursor as isize - 1);

    let tty = File::options().read(true).write(true).open("/dev/tty").context("open /dev/tty")?;
    match pick(&tty, &mut st, &o)? {
        Outcome::Accept(key) => print_result(&st, &o, key),
        Outcome::Abort => Ok(130),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_ranks_boundaries_and_smart_case() {
        assert!(fuzzy("~/Projects/foo", "pf").is_some());
        assert!(fuzzy("abc", "abd").is_none());
        assert!(fuzzy("abc", "A").is_none());
        let (word, _) = fuzzy("src/dots", "dots").unwrap();
        let (mid, _) = fuzzy("xxdxoxtxs", "dots").unwrap();
        assert!(word > mid);
        assert_eq!(fuzzy("a-b", "ab").unwrap().1, vec![0, 2]);
    }

    #[test]
    fn toggle_clears_query_and_keeps_item_under_cursor() {
        let items = ["alpha", "beta", "gamma"].map(|s| (s.to_string(), s.to_string())).to_vec();
        let mut st = State { items, styles: vec![], query: "gam".into(), matches: vec![], checked: vec![false; 3], selected: 0, offset: 0, page: 1 };
        st.refilter();
        st.toggle();
        assert_eq!(st.checked, [false, false, true]);
        assert!(st.query.is_empty());
        assert_eq!(st.current(), Some("gamma"));
        st.toggle();
        assert_eq!(st.checked, [false; 3]);
    }

    #[test]
    fn ansi_strips_and_colors() {
        let (text, styles) = strip_ansi("\x1b[31mab\x1b[0mc\x1b[38;5;200md");
        assert_eq!(text, "abcd");
        assert_eq!(styles[1].fg, Some(Color::Indexed(1)));
        assert_eq!(styles[2].fg, None);
        assert_eq!(styles[3].fg, Some(Color::Indexed(200)));
    }

    #[test]
    fn keys_parse() {
        assert_eq!(parse_key("del").unwrap().0, KeyCode::Delete);
        assert_eq!(parse_key("ctrl-d").unwrap(), (KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(parse_key("hyper-x").is_err());
    }

    #[test]
    fn fzf_opts_parse() {
        assert_eq!(shell_words(r#"--prompt='> ' --color a:b "x\"y" z"#), ["--prompt=> ", "--color", "a:b", "x\"y", "z"]);
        let mut t = Theme::default();
        t.apply("bg:-1,bg+:#221326,hl:#4eb0d0:bold,dark,pointer:5");
        assert_eq!(t.bg.bg, Some(Color::Reset));
        assert_eq!(t.bg_sel.bg, Some(Color::Rgb(0x22, 0x13, 0x26)));
        assert!(t.hl.add_modifier.contains(Modifier::BOLD));
        assert_eq!(t.pointer.fg, Some(Color::Indexed(5)));
    }
}
