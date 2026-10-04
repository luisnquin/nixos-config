use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use crate::adb::{self, Server};
use crate::doctor::motion::{not_idle, Pin, Scales};
use crate::simctl;

/// uiautomator will not write to stdout on every vendor build, so the dump goes
/// to a file that is read and removed in the same shell. `--windows`, absent from
/// its usage text, dumps every window rather than the focused one, which is
/// where an overlay or a popup the app draws in a window of its own lives.
const DUMP: &str = "said=$(uiautomator dump --windows /sdcard/.phone-a11y.xml 2>&1); \
     case \"$said\" in *'could not get idle state'*) echo phone:not-idle; \
     echo \"phone:scales $(settings get global window_animation_scale) \
     $(settings get global transition_animation_scale) \
     $(settings get global animator_duration_scale)\";; esac";

const READ: &str = "cat /sdcard/.phone-a11y.xml 2>/dev/null; rm -f /sdcard/.phone-a11y.xml";

const KEYBOARD: &str = "dumpsys input_method 2>/dev/null | grep -m1 mInputShown; \
     dumpsys window 2>/dev/null | grep -m1 'type=ime frame='";

const PANEL: &str = "dumpsys window displays 2>/dev/null | grep -E 'mDisplayId=| cur=[0-9]'";

const NOT_IDLE: &str = "phone:not-idle";

#[derive(Debug)]
pub struct NotIdle {
    scales: Option<Scales>,
    pin: Option<Pin>,
}

impl std::fmt::Display for NotIdle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&not_idle(self.scales, self.pin.as_ref()))
    }
}

impl std::error::Error for NotIdle {}

/// A device to read and press, resolved once per invocation. The two arms differ
/// only in how a verb reaches the device: the elements they report and the
/// coordinates they take are the same shape either way.
pub enum Target {
    Adb(Adb),
    Simulator(Simulator),
}

pub struct Adb {
    pub server: Server,
    pub serial: String,
    pub display: Option<adb::Display>,
    pub focus: Option<(i32, i32)>,
    pub device: Box<crate::model::Device>,
}

/// CoreSimulator is macOS-local, so the verbs run on the machine that owns the
/// simulator rather than over a transport pointed at it — which is this one when
/// the simulator is here.
pub struct Simulator {
    pub at: crate::ssh::Where,
    pub udid: String,
}

impl Adb {
    /// The dump and every key go to whichever window holds focus, so a press
    /// that pulls it has to run in the same shell, where nothing can interleave.
    fn prefix(&self) -> String {
        self.focus
            .map(|(x, y)| format!("input tap {x} {y}; sleep 0.6; "))
            .unwrap_or_default()
    }

    fn input(&self) -> String {
        let aim = self
            .display
            .map(|d| format!(" -d {}", d.logical))
            .unwrap_or_default();

        format!("input{aim}")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Bounds {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Bounds {
    fn parse(raw: &str) -> Option<Self> {
        let (a, b) = raw.trim_start_matches('[').split_once("][")?;
        let (x1, y1) = a.split_once(',')?;
        let (x2, y2) = b.trim_end_matches(']').split_once(',')?;

        Some(Bounds {
            x1: x1.parse().ok()?,
            y1: y1.parse().ok()?,
            x2: x2.parse().ok()?,
            y2: y2.parse().ok()?,
        })
    }

    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }

    /// Grown by `pad` on every side, clamped to the panel it came from.
    pub fn padded(&self, pad: i32, within: Option<(i32, i32)>) -> Self {
        let (w, h) = within.unwrap_or((i32::MAX, i32::MAX));

        Bounds {
            x1: (self.x1 - pad).max(0),
            y1: (self.y1 - pad).max(0),
            x2: (self.x2 + pad).min(w),
            y2: (self.y2 + pad).min(h),
        }
    }

    pub fn width(&self) -> i32 {
        (self.x2 - self.x1).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.y2 - self.y1).max(0)
    }

    fn area(&self) -> i64 {
        ((self.x2 - self.x1) as i64).max(0) * ((self.y2 - self.y1) as i64).max(0)
    }

    fn contains(&self, other: &Bounds) -> bool {
        self.x1 <= other.x1 && self.y1 <= other.y1 && self.x2 >= other.x2 && self.y2 >= other.y2
    }

    pub fn holds(&self, (x, y): (i32, i32)) -> bool {
        (self.x1..self.x2).contains(&x) && (self.y1..self.y2).contains(&y)
    }

    pub fn clipped(&self, to: &Bounds) -> Option<Bounds> {
        let clip = Bounds {
            x1: self.x1.max(to.x1),
            y1: self.y1.max(to.y1),
            x2: self.x2.min(to.x2),
            y2: self.y2.min(to.y2),
        };

        (clip.area() > 0).then_some(clip)
    }
}

/// Where an element can be pressed. A window parked mostly off the panel, like
/// a picture-in-picture stashed against an edge, has its centre out of reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    On((i32, i32)),
    Partly {
        at: (i32, i32),
        edge: &'static str,
        showing: i32,
    },
    Off,
}

impl Reach {
    pub fn of(bounds: Bounds, panel: Option<Bounds>) -> Self {
        let centre = bounds.center();

        let Some(panel) = panel.filter(|p| !p.holds(centre)) else {
            return Reach::On(centre);
        };

        let Some(seen) = bounds.clipped(&panel) else {
            return Reach::Off;
        };

        let (edge, _, showing) = [
            ("left", panel.x1 - bounds.x1, seen.width()),
            ("right", bounds.x2 - panel.x2, seen.width()),
            ("top", panel.y1 - bounds.y1, seen.height()),
            ("bottom", bounds.y2 - panel.y2, seen.height()),
        ]
        .into_iter()
        .max_by_key(|(_, over, _)| *over)
        .expect("four edges");

        Reach::Partly {
            at: seen.center(),
            edge,
            showing,
        }
    }

    pub fn point(&self) -> Option<(i32, i32)> {
        match *self {
            Reach::On(at) | Reach::Partly { at, .. } => Some(at),
            Reach::Off => None,
        }
    }

    pub fn note(&self) -> Option<String> {
        match self {
            Reach::On(_) => None,
            Reach::Partly { edge, showing, .. } => {
                Some(format!("mostly off the {edge} edge, {showing}px showing"))
            }
            Reach::Off => Some("off the panel".to_string()),
        }
    }
}

fn panel_of(text: &str, display: u32) -> Option<Bounds> {
    let wanted = format!("mDisplayId={display} ");
    let mut current = false;

    for line in text.lines() {
        if line.contains("mDisplayId=") {
            current = line.contains(&wanted) || line.trim_end().ends_with(wanted.trim_end());
            continue;
        }

        let Some((_, rest)) = line.split_once(" cur=").filter(|_| current) else {
            continue;
        };
        let (w, h) = rest.split_whitespace().next()?.split_once('x')?;

        return Some(Bounds {
            x1: 0,
            y1: 0,
            x2: w.parse().ok()?,
            y2: h.parse().ok()?,
        });
    }

    None
}

/// The panel in the space its element bounds and taps are given in, and the
/// factor that maps that space to the pixels a screenshot comes back in.
/// Android reports both in pixels, so `scale` is 1 there; a simulator reports
/// points and screenshots at 2x or 3x, which is what makes a crop taken from
/// element bounds land somewhere else entirely.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

impl Size {
    /// Where `bounds` falls in the screenshot.
    pub fn in_pixels(self, bounds: Bounds) -> Bounds {
        let at = |v: i32| (f64::from(v) * self.scale).round() as i32;

        Bounds {
            x1: at(bounds.x1),
            y1: at(bounds.y1),
            x2: at(bounds.x2),
            y2: at(bounds.y2),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    /// Stable only within one dump.
    pub index: usize,
    pub text: String,
    pub desc: String,
    pub res_id: String,
    pub class: String,
    pub clickable: bool,
    pub bounds: Bounds,
    /// Every box this element sits inside, nearest first. What a control looks
    /// like is mostly drawn by its container — the row, the card, the border —
    /// so a crop of the element alone is a crop of the label on it.
    ///
    /// Absent from a snapshot taken by a receiver older than the field, which
    /// is why the callers say so rather than quietly cropping to nothing.
    #[serde(default)]
    pub ancestors: Vec<Bounds>,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub password: bool,
    #[serde(default)]
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Signature {
    pub res_id: String,
    pub class: String,
    pub text: String,
    pub desc: String,
    pub bounds: Bounds,
}

impl Node {
    pub fn signature(&self) -> Signature {
        Signature {
            res_id: self.res_id.clone(),
            class: self.class.clone(),
            text: self.text.clone(),
            desc: self.desc.clone(),
            bounds: self.bounds,
        }
    }

    /// An empty field reports its hint as its text.
    pub fn is_empty_field(&self) -> bool {
        self.text.is_empty() || (!self.hint.is_empty() && self.text == self.hint)
    }

    pub fn reads(&self, wanted: &str) -> bool {
        match wanted.is_empty() {
            true => self.is_empty_field(),
            false => !self.is_empty_field() && self.text == wanted,
        }
    }

    pub fn describe_field(&self) -> String {
        let name = self.field_name();

        let value = if self.password {
            "password".to_string()
        } else if self.is_empty_field() {
            match self.hint.as_str() {
                "" => "empty".to_string(),
                hint => format!("empty, hint {hint:?}"),
            }
        } else {
            format!("{:?}", self.text)
        };

        format!("{name} ({}) {value}", self.kind())
    }

    pub fn field_name(&self) -> String {
        match self.res_id.as_str() {
            "" => self.label(),
            id => id.to_string(),
        }
    }

    /// The name this element answers to, empty when it carries none of its own.
    /// Deliberately the same three fields `matches` searches: what a snapshot
    /// prints and what `pick` can resolve have to be one set, or a caller reads
    /// a name off the screen and presses at nothing.
    pub fn name(&self) -> &str {
        for candidate in [&self.text, &self.desc, &self.res_id] {
            if !candidate.is_empty() {
                return candidate;
            }
        }

        ""
    }

    /// The class, without the package every node of one app shares.
    pub fn kind(&self) -> &str {
        self.class.rsplit('.').next().unwrap_or(&self.class)
    }

    /// What to print. A nameless element is shown as its class in angle
    /// brackets, which reads as a description rather than as a name — the
    /// point being that `<View>` is the one form `pick` will never resolve, so
    /// the `@index` printed beside it is visibly the only way to reach the row.
    pub fn label(&self) -> String {
        match self.name() {
            "" => format!("<{}>", self.kind()),
            name => name.to_string(),
        }
    }

    /// The `levels`th box around this element that is actually bigger than it.
    /// Layout puts a stack of wrappers on the same rectangle, and counting
    /// those would make `--expand 1` land back on the element it started from.
    pub fn enclosing(&self, levels: usize) -> Option<Bounds> {
        self.boxes().nth(levels.saturating_sub(1))
    }

    /// How far `--expand` can go before it runs out of screen.
    pub fn enclosures(&self) -> usize {
        self.boxes().count()
    }

    fn boxes(&self) -> impl Iterator<Item = Bounds> + '_ {
        self.ancestors
            .iter()
            .copied()
            .filter(|a| a.area() > self.bounds.area())
    }

    pub fn matches(&self, needle: &str) -> bool {
        let needle = folded(needle);

        [&self.text, &self.desc, &self.res_id]
            .iter()
            .any(|field| folded(field).contains(&needle))
    }

    pub fn answers(&self, needle: &str) -> bool {
        let needle = folded(needle);

        [&self.text, &self.desc, &self.res_id]
            .iter()
            .any(|field| folded(field) == needle)
    }
}

/// Apps typeset their labels ("Wi‑Fi" has a non-breaking hyphen, "Don’t" a curly
/// apostrophe) and nobody types those.
fn folded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{2010}'..='\u{2015}' | '\u{2212}' => '-',
            '\u{00a0}' | '\u{2007}' | '\u{2009}' | '\u{202f}' => ' ',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// The elements that can be read or pressed. The rest of the hierarchy is
/// layout containers, which are most of the several hundred nodes a dump holds.
pub fn parse(xml: &str) -> Result<Vec<Node>> {
    let doc = roxmltree::Document::parse(xml)?;
    let mut nodes = Vec::new();

    // walked rather than iterated flat, because a kept element needs the boxes
    // it sits inside and most of those are containers that are dropped
    for hierarchy in doc
        .descendants()
        .filter(|e| e.has_tag_name("hierarchy"))
        .filter(|e| {
            e.ancestors()
                .find(|a| a.has_tag_name("window"))
                .is_none_or(readable)
        })
    {
        walk(hierarchy, &[], None, &mut nodes);
    }

    Ok(nodes)
}

fn readable(window: roxmltree::Node) -> bool {
    let bar = window
        .attribute("bounds")
        .and_then(Bounds::parse)
        .is_some_and(|b| b.x1 == 0 && b.height() * 6 < b.width());

    match window.attribute("type") {
        Some("TYPE_INPUT_METHOD") => false,
        Some("TYPE_SYSTEM") => !bar,
        _ => true,
    }
}

fn walk(
    element: roxmltree::Node,
    enclosing: &[Bounds],
    mut parent: Option<usize>,
    out: &mut Vec<Node>,
) {
    let attr = |name| element.attribute(name).unwrap_or_default().to_string();
    let bounds = element.attribute("bounds").and_then(Bounds::parse);

    if let Some(bounds) = bounds.filter(|b| b.area() > 0) {
        let text = attr("text");
        let desc = attr("content-desc");
        let res_id = attr("resource-id");
        let clickable = element.attribute("clickable") == Some("true");
        let focused = element.attribute("focused") == Some("true");

        if !(text.is_empty() && desc.is_empty() && !clickable && !focused) {
            out.push(Node {
                index: out.len(),
                text,
                desc,
                // the package prefix is identical for every node of one app
                res_id: res_id.rsplit('/').next().unwrap_or(&res_id).to_string(),
                class: attr("class"),
                clickable,
                bounds,
                ancestors: enclosing.to_vec(),
                focused,
                hint: attr("hint"),
                password: element.attribute("password") == Some("true"),
                parent,
            });

            parent = Some(out.len() - 1);
        }
    }

    // a run of wrappers on one rectangle is one box to a reader, and keeping
    // each of them would make `--expand` count layout rather than structure
    let nested;
    let enclosing = match bounds {
        Some(b) if enclosing.first() != Some(&b) => {
            nested = [&[b][..], enclosing].concat();
            &nested
        }
        _ => enclosing,
    };

    for child in element.children().filter(|c| c.has_tag_name("node")) {
        walk(child, enclosing, parent, out);
    }
}

/// uiautomator loses to a window that is still being laid out and answers with
/// nothing at all rather than with a partial tree, so a dump taken right after a
/// tap is worth asking for twice before calling the screen unreadable.
const DUMP_TRIES: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keyboard {
    pub shown: bool,
    pub frame: Option<Bounds>,
}

impl Keyboard {
    fn parse(text: &str) -> Option<Self> {
        let shown = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("mInputShown="))?
            == "true";

        let frame = text
            .lines()
            .find_map(|line| line.split_once("type=ime frame="))
            .and_then(|(_, rest)| Bounds::parse(rest.split_whitespace().next()?))
            .filter(|b| shown && b.area() > 0);

        Some(Keyboard { shown, frame })
    }

    pub fn describe(&self) -> String {
        match (self.shown, self.frame) {
            (false, _) => "down".to_string(),
            (true, None) => "up".to_string(),
            (true, Some(b)) => format!("up over [{},{}][{},{}]", b.x1, b.y1, b.x2, b.y2),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Screen {
    pub nodes: Vec<Node>,
    pub keyboard: Option<Keyboard>,
    pub panel: Option<Bounds>,
}

impl Screen {
    pub fn covered(&self, node: &Node) -> bool {
        let at = self.reach(node).point();

        self.keyboard
            .and_then(|k| k.frame)
            .zip(at)
            .is_some_and(|(frame, at)| frame.holds(at))
    }

    pub fn reach(&self, node: &Node) -> Reach {
        Reach::of(node.bounds, self.panel)
    }

    pub fn focused(&self) -> Option<&Node> {
        self.nodes.iter().find(|n| n.focused)
    }
}

pub async fn dump(t: &Target) -> Result<Screen> {
    let _spent = crate::calls::time(crate::calls::Cost::Dump);
    let screen = dumped(t).await?;

    crate::calls::focused_plain(screen.focused().is_some_and(|n| !n.password));

    Ok(screen)
}

async fn dumped(t: &Target) -> Result<Screen> {
    let a = match t {
        Target::Adb(a) => a,
        Target::Simulator(s) => {
            return Ok(Screen {
                nodes: simctl::snapshot(&s.at, &s.udid).await?,
                keyboard: None,
                panel: None,
            })
        }
    };

    let mut last = None;

    for attempt in 0..DUMP_TRIES {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(600)).await;
        }

        match dump_once(a).await.map_err(|e| e.downcast::<NotIdle>()) {
            Ok(screen) => return Ok(screen),
            Err(Ok(stuck)) => {
                return Err(NotIdle {
                    pin: stuck.scales.and_then(|_| Pin::of(&a.device)),
                    ..stuck
                }
                .into())
            }
            Err(Err(e)) => last = Some(e),
        }
    }

    Err(last.expect("the loop runs at least once"))
}

struct Reader {
    local: &'static str,
    remote: String,
}

fn reader() -> Option<Reader> {
    let local = option_env!("PHONE_DUMP_DEX")?;
    let sum = std::fs::read(local)
        .ok()?
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3));

    Some(Reader {
        local,
        remote: format!("/data/local/tmp/phone-dump-{sum:016x}.dex"),
    })
}

const NO_READER: &str = "phone:no-reader";

async fn dump_once(a: &Adb) -> Result<Screen> {
    let mut unread = None;

    if let Some(reader) = reader().filter(|_| a.display.is_none_or(|d| d.logical == 0)) {
        match read_with(a, &reader).await {
            Ok(screen) => return Ok(screen),
            Err(e) => unread = Some(e),
        }
    }

    let remote = format!("{}{DUMP}; {KEYBOARD}; {PANEL}; {READ}", a.prefix());
    let (ok, bytes) = adb::run_bytes(&a.server, &["-s", &a.serial, "exec-out", &remote]).await?;

    match read_dump(ok, logical(a), &String::from_utf8_lossy(&bytes)) {
        Err(e) if e.is::<NotIdle>() => Err(unread.unwrap_or(e)),
        read => read,
    }
}

async fn read_with(a: &Adb, reader: &Reader) -> Result<Screen> {
    let (ok, bytes) = on_helper(a, reader, |prefix| {
        format!("{prefix}{KEYBOARD}; {PANEL}; {}", launch(reader, "2>/dev/null"))
    })
    .await?;

    read_dump(ok, logical(a), &String::from_utf8_lossy(&bytes))
}

const TOUCHED: &str = "phone:touched";

fn launch(reader: &Reader, args: &str) -> String {
    format!(
        "if [ -f {0} ]; then CLASSPATH={0} app_process /system/bin PhoneDump {args}; \
         else echo {NO_READER}; fi",
        reader.remote
    )
}

async fn on_helper(a: &Adb, reader: &Reader, run: impl Fn(String) -> String) -> Result<(bool, Vec<u8>)> {
    let (ok, bytes) = adb::run_bytes(&a.server, &["-s", &a.serial, "exec-out", &run(a.prefix())]).await?;

    if !String::from_utf8_lossy(&bytes).contains(NO_READER) {
        return Ok((ok, bytes));
    }

    let pushed = adb::run(&a.server, &["-s", &a.serial, "push", reader.local, &reader.remote]).await?;
    if !pushed.ok() {
        bail!("pushing the screen reader: {}", pushed.stderr.trim());
    }

    adb::run_bytes(&a.server, &["-s", &a.serial, "exec-out", &run(String::new())]).await
}

fn read_dump(ok: bool, display: u32, out: &str) -> Result<Screen> {
    let (said, xml) = out.split_at(
        out.find("<?xml")
            .or_else(|| out.find("<hierarchy"))
            .unwrap_or(out.len()),
    );

    if said.contains(NOT_IDLE) {
        return Err(NotIdle {
            scales: Scales::parse(said),
            pin: None,
        }
        .into());
    }

    if !ok || !xml.contains("<hierarchy") {
        bail!("uiautomator returned no hierarchy (is the screen on and unlocked?)");
    }

    Ok(Screen {
        nodes: parse(xml)?,
        keyboard: Keyboard::parse(said),
        panel: panel_of(said, display),
    })
}

fn logical(a: &Adb) -> u32 {
    a.display.map_or(0, |d| d.logical)
}

fn record_path(device: &str) -> PathBuf {
    let name: String = device
        .chars()
        .map(
            |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                true => c,
                false => '_',
            },
        )
        .collect();

    crate::registry::state_dir()
        .join("snapshots")
        .join(format!("{name}.json"))
}

pub fn remember(device: &str, nodes: &[Node]) -> Result<()> {
    remember_rows(
        device,
        &nodes.iter().map(Node::signature).collect::<Vec<_>>(),
    )
}

pub fn remember_rows(device: &str, rows: &[Signature]) -> Result<()> {
    let path = record_path(device);

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    std::fs::write(&path, serde_json::to_vec(rows)?)?;

    Ok(())
}

pub fn recall(device: &str) -> Option<Vec<Signature>> {
    serde_json::from_slice(&std::fs::read(record_path(device)).ok()?).ok()
}

/// The panel, in the space `bounds` and taps use.
pub async fn size(t: &Target) -> Result<Size> {
    match t {
        Target::Adb(a) => {
            let (width, height) = adb::screen_size(&a.server, &a.serial, a.display)
                .await
                .ok_or_else(|| anyhow::anyhow!("could not read the panel size"))?;

            Ok(Size {
                width: f64::from(width),
                height: f64::from(height),
                scale: 1.0,
            })
        }
        Target::Simulator(s) => simctl::size(&s.at, &s.udid).await,
    }
}

/// The one element `needle` names. Ambiguity is reported rather than resolved by
/// picking the first, since acting on the wrong control is worse than not acting.
#[cfg(test)]
fn pick<'a>(nodes: &'a [Node], needle: &str) -> Result<&'a Node> {
    pick_in(nodes, needle, None)
}

#[derive(Debug)]
pub struct Ambiguous(String);

impl std::fmt::Display for Ambiguous {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Ambiguous {}

#[derive(Debug)]
pub struct Missing(String);

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Missing {}

pub fn pick_in<'a>(
    nodes: &'a [Node],
    needle: &str,
    shown: Option<&[Signature]>,
) -> Result<&'a Node> {
    if let Some(index) = needle
        .strip_prefix('@')
        .and_then(|n| n.parse::<usize>().ok())
    {
        let Some(shown) = shown else {
            return nodes
                .get(index)
                .ok_or_else(|| anyhow!("no element @{index} in this snapshot"));
        };

        let row = shown
            .get(index)
            .ok_or_else(|| anyhow!("no element @{index} in the last snapshot"))?;

        return found(nodes, row).ok_or_else(|| moved(nodes, index, row));
    }

    let hits: Vec<&Node> = nodes.iter().filter(|n| n.matches(needle)).collect();
    let exact: Vec<&Node> = hits.iter().copied().filter(|n| n.answers(needle)).collect();

    match (hits.as_slice(), exact.as_slice()) {
        ([], _) => Err(Missing(format!(
            "nothing on screen matches '{needle}'{}",
            near(nodes, needle)
        ))
        .into()),
        // a name that used to be on screen reads as part of a longer one that
        // still is, and pressing that one is pressing the wrong thing
        (partial, []) => Err(Ambiguous(format!(
            "nothing on screen is named '{needle}', it is only part of {}; use a whole name or its @index",
            listed(partial)
        ))
        .into()),
        (_, [one]) => Ok(one),
        (many, _) => {
            // a pressable and the label drawn inside it both carry the name,
            // and pressing either presses the same thing
            if let Some(outer) = outermost(many).or_else(|| outermost(&exact)) {
                return Ok(outer);
            }

            if let [one] = many
                .iter()
                .copied()
                .filter(|n| n.clickable)
                .collect::<Vec<_>>()
                .as_slice()
            {
                return Ok(one);
            }

            Err(Ambiguous(format!(
                "'{needle}' matches {} elements: {}",
                many.len(),
                listed(many)
            ))
            .into())
        }
    }
}

/// Pointed at by name rather than by a fresh `@index`: renumbering the kept rows
/// here would silently re-aim every other index the caller still holds.
fn moved(nodes: &[Node], index: usize, row: &Signature) -> anyhow::Error {
    let same = |n: &Node| {
        (&n.res_id, &n.class, &n.text, &n.desc) == (&row.res_id, &row.class, &row.text, &row.desc)
    };

    match [&row.text, &row.desc, &row.res_id]
        .into_iter()
        .find(|s| !s.is_empty())
    {
        Some(name) if pick_in(nodes, name, None).is_ok_and(same) => {
            anyhow!("@{index} ({name}) moved; it is still on screen, name it '{name}' instead")
        }
        name => anyhow!(
            "@{index} ({}) is no longer where the last snapshot saw it; take a new snapshot{}",
            name.cloned().unwrap_or_else(|| format!(
                "<{}>",
                row.class.rsplit('.').next().unwrap_or_default()
            )),
            match row.res_id.is_empty() {
                true => "; it has no id (testID in React Native), so only its exact text and place find it",
                false => "",
            }
        ),
    }
}

/// Names on screen a typo or an accent away from `needle`. Only near spellings:
/// the nearest of unrelated names would send a retry at a different control.
pub fn near(nodes: &[Node], needle: &str) -> String {
    let want: Vec<char> = folded(needle).chars().collect();
    let limit = want.len() / 4;
    let digits = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();

    let mut close: Vec<(usize, &str)> = nodes
        .iter()
        .flat_map(|n| [&n.text, &n.desc, &n.res_id])
        .filter(|name| !name.is_empty() && digits(name) == digits(needle))
        .map(|name| (distance(&want, &folded(name)), name.as_str()))
        .filter(|(d, _)| *d <= limit)
        .collect();

    close.sort_unstable();
    close.dedup();

    match close
        .iter()
        .take(3)
        .map(|(_, name)| format!("'{name}'"))
        .collect::<Vec<_>>()
    {
        names if names.is_empty() => String::new(),
        names => format!("; close: {}", names.join(", ")),
    }
}

fn distance(a: &[char], b: &str) -> usize {
    let mut row: Vec<usize> = (0..=a.len()).collect();

    for (j, cb) in b.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = j + 1;

        for (i, ca) in a.iter().enumerate() {
            let next = (row[i + 1] + 1)
                .min(row[i] + 1)
                .min(diagonal + usize::from(*ca != cb));
            diagonal = row[i + 1];
            row[i + 1] = next;
        }
    }

    row[a.len()]
}

fn listed(nodes: &[&Node]) -> String {
    nodes
        .iter()
        .map(|n| format!("@{} {}", n.index, n.label()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn outermost<'a>(hits: &[&'a Node]) -> Option<&'a Node> {
    hits.iter()
        .copied()
        .filter(|n| n.clickable)
        .max_by_key(|n| n.bounds.area())
        .filter(|outer| hits.iter().all(|n| outer.bounds.contains(&n.bounds)))
}

/// Text and size are state as much as identity, so an id only one element on
/// screen carries is enough; without one, only its words and place are left,
/// numbers aside: a latency or a progress counter ticks between snapshot and tap.
fn found<'a>(nodes: &'a [Node], row: &Signature) -> Option<&'a Node> {
    if let Some(same) = nodes.iter().find(|n| n.signature() == *row) {
        return Some(same);
    }

    let ticked = |n: &&Node| {
        n.res_id.is_empty()
            && (&n.class, n.bounds) == (&row.class, row.bounds)
            && masked(&n.text) == masked(&row.text)
            && masked(&n.desc) == masked(&row.desc)
    };
    let same_id = |n: &&Node| n.res_id == row.res_id && n.class == row.class;

    match nodes
        .iter()
        .filter(|n| match row.res_id.is_empty() {
            true => ticked(n),
            false => same_id(n),
        })
        .collect::<Vec<_>>()
        .as_slice()
    {
        [one] => Some(one),
        _ => None,
    }
}

fn masked(s: &str) -> String {
    let mut out = String::with_capacity(s.len());

    for c in s.chars() {
        match c.is_ascii_digit() {
            true if out.ends_with('\0') => {}
            true => out.push('\0'),
            false => out.push(c),
        }
    }

    out
}

/// Whether anything on screen answers to `needle`. Unlike `pick`, how many do
/// is not the question: two matches still means it is there. `@index` is not
/// accepted, and callers reject it before asking — it numbers the rows of one
/// dump, so across two it answers about the length of a list, not about a thing.
pub fn present(nodes: &[Node], needle: &str) -> bool {
    nodes.iter().any(|n| n.matches(needle))
}

pub struct Row<'a> {
    pub node: &'a Node,
    pub label: String,
    pub within: Option<usize>,
}

/// A folded row and a borrowed name both stay reachable by name and by their own
/// `@index`; only the table leaves them out.
pub fn rows(nodes: &[Node]) -> Vec<Row<'_>> {
    let mut kids = vec![Vec::new(); nodes.len()];

    for (i, n) in nodes.iter().enumerate() {
        if let Some(p) = n.parent.filter(|p| *p < i) {
            kids[p].push(i);
        }
    }

    let names: Vec<&str> = nodes
        .iter()
        .zip(&kids)
        .map(|(n, kids)| match kids.as_slice() {
            [only] if n.clickable && n.name().is_empty() && loose(&nodes[*only]) => {
                nodes[*only].name()
            }
            _ => n.name(),
        })
        .collect();

    let mut within: Vec<Option<usize>> = Vec::with_capacity(nodes.len());

    for (i, n) in nodes.iter().enumerate() {
        let host = n.parent.filter(|p| {
            *p < i && loose(n) && !names[*p].is_empty() && first_line(names[*p]).contains(n.name())
        });

        within.push(host.map(|p| within[p].unwrap_or(p)));
    }

    nodes
        .iter()
        .zip(names)
        .zip(within)
        .map(|((node, name), within)| Row {
            node,
            label: match name {
                "" => node.label(),
                name => name.to_string(),
            },
            within,
        })
        .collect()
}

fn loose(n: &Node) -> bool {
    !n.clickable && !n.focused && !n.name().is_empty()
}

const ROW_LIMIT: usize = 100;

pub fn row_label(label: &str) -> String {
    let lines = label.lines().count();
    let mut row = first_line(label);

    if lines > 1 {
        row.push_str(&format!(" (+{} lines)", lines - 1));
    }

    row
}

fn first_line(label: &str) -> String {
    let first = label.lines().next().unwrap_or_default();
    let mut row: String = first.chars().take(ROW_LIMIT).collect();

    if row.len() < first.len() {
        row.push('…');
    }

    row
}

/// Sent as one device-side command, because `input text` reads the rest of the
/// line as its own words and flags. Without `-d` it aims at logical display 0,
/// which is the live panel on everything that has one.
async fn input(a: &Adb, args: &str) -> Result<()> {
    shell(a, &format!("{} {args}", a.input())).await
}

const SHELL_TIMEOUT: Duration = Duration::from_secs(20);

async fn shell(a: &Adb, script: &str) -> Result<()> {
    shell_within(a, script, SHELL_TIMEOUT).await
}

async fn shell_within(a: &Adb, script: &str, limit: Duration) -> Result<()> {
    let remote = format!("{}{script}", a.prefix());
    let out = adb::run_timeout(&a.server, &["-s", &a.serial, "shell", &remote], limit).await?;

    if out.ok() {
        Ok(())
    } else {
        bail!("{}", out.stderr.trim())
    }
}

pub async fn tap(t: &Target, x: i32, y: i32) -> Result<()> {
    match t {
        Target::Adb(a) => input(a, &format!("tap {x} {y}")).await,
        Target::Simulator(s) => simctl::tap(&s.at, &s.udid, x, y).await,
    }
}

/// A drag from one point to the other over `hold`. A long press is the same
/// gesture with both ends in one place: what a device tells apart is how long a
/// touch lasts, not how far it travelled.
pub async fn swipe(t: &Target, from: (i32, i32), to: (i32, i32), hold: Duration) -> Result<()> {
    let ms = hold.as_millis().max(1);
    let ((x1, y1), (x2, y2)) = (from, to);

    match t {
        Target::Adb(a) => input(a, &format!("swipe {x1} {y1} {x2} {y2} {ms}")).await,
        Target::Simulator(s) => simctl::swipe(&s.at, &s.udid, from, to, ms as u64).await,
    }
}

/// `input swipe` moves at once and `input draganddrop` holds only for the
/// system's long-press timeout, so the hold is spelled out in motionevents.
pub async fn drag(
    t: &Target,
    from: (i32, i32),
    to: (i32, i32),
    hold: Duration,
    over: Duration,
) -> Result<()> {
    let Target::Adb(a) = t else {
        bail!("a held swipe needs Android; a simulator only takes one that moves at once");
    };

    let script = drag_script(&a.input(), from, to, hold, over);

    shell_within(a, &script, SHELL_TIMEOUT + hold + over).await
}

fn drag_script(
    input: &str,
    (x1, y1): (i32, i32),
    (x2, y2): (i32, i32),
    hold: Duration,
    over: Duration,
) -> String {
    let steps = (over.as_millis() / 100).clamp(1, 10) as i32;
    let pause = over.as_secs_f64() / f64::from(steps);

    let mut script = format!(
        "{input} motionevent DOWN {x1} {y1}; sleep {:.3}",
        hold.as_secs_f64()
    );

    for i in 1..=steps {
        let x = x1 + (x2 - x1) * i / steps;
        let y = y1 + (y2 - y1) * i / steps;

        script.push_str(&format!(
            "; {input} motionevent MOVE {x} {y}; sleep {pause:.3}"
        ));
    }

    script.push_str(&format!("; {input} motionevent UP {x2} {y2}"));

    script
}

/// Where a swipe in `direction` starts and ends. It runs through the middle of
/// the panel, over `amount` of its length, and stays inside the margins: a drag
/// begun at the very edge is a system gesture — back, notifications, app switch
/// — and never reaches the app.
pub fn along(size: Size, direction: Direction, amount: f64) -> ((i32, i32), (i32, i32)) {
    let amount = amount.clamp(0.05, 0.8);
    let (w, h) = (size.width, size.height);

    let (span, mid) = match direction {
        Direction::Up | Direction::Down => (h, w / 2.0),
        Direction::Left | Direction::Right => (w, h / 2.0),
    };

    let travel = span * amount;
    let (near, far) = ((span - travel) / 2.0, (span + travel) / 2.0);

    let point = |along: f64| match direction {
        Direction::Up | Direction::Down => (mid.round() as i32, along.round() as i32),
        Direction::Left | Direction::Right => (along.round() as i32, mid.round() as i32),
    };

    // the finger moves the way it is named, so the content follows it
    match direction {
        Direction::Up | Direction::Left => (point(far), point(near)),
        Direction::Down | Direction::Right => (point(near), point(far)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl std::str::FromStr for Direction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            "left" => Ok(Direction::Left),
            "right" => Ok(Direction::Right),
            other => bail!("'{other}' is not a direction (up, down, left, right)"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gesture {
    pub centre: [(f64, f64); 2],
    pub span: [f64; 2],
    pub turn: [f64; 2],
    pub fingers: usize,
    pub taps: usize,
}

type Rect = (f64, f64, f64, f64);

struct Room {
    centre: (f64, f64),
    area: Rect,
    inside: Rect,
    gap: f64,
}

const FINGER_EDGE: f64 = 0.05;
const FINGER_GAP: f64 = 0.08;
const FINGER_SPACING: f64 = 0.12;
const GESTURE_STEP: Duration = Duration::from_millis(16);
const GESTURE_STEPS_MAX: usize = 60;
pub const TAP_GAP: Duration = Duration::from_millis(100);
pub const TAP_DOWN: Duration = Duration::from_millis(50);

impl Gesture {
    pub fn at(&self, t: f64) -> Vec<(i32, i32)> {
        let lerp = |[a, b]: [f64; 2]| a + (b - a) * t;
        let [(x1, y1), (x2, y2)] = self.centre;
        let (cx, cy) = (lerp([x1, x2]), lerp([y1, y2]));
        let (span, turn) = (lerp(self.span), lerp(self.turn).to_radians());
        let last = self.fingers.saturating_sub(1).max(1) as f64;

        (0..self.fingers)
            .map(|i| {
                let off = match self.fingers {
                    1 => 0.0,
                    _ => span * (i as f64 / last - 0.5),
                };

                ((cx + off * turn.cos()).round() as i32, (cy + off * turn.sin()).round() as i32)
            })
            .collect()
    }

    pub fn frames(&self, over: Duration) -> Vec<Vec<(i32, i32)>> {
        let steps = ((over.as_millis() / GESTURE_STEP.as_millis()) as usize).clamp(2, GESTURE_STEPS_MAX);

        (0..=steps).map(|i| self.at(i as f64 / steps as f64)).collect()
    }

    pub fn gap(&self) -> (i32, i32) {
        (self.span[0].round() as i32, self.span[1].round() as i32)
    }
}

impl Room {
    fn new(centre: (i32, i32), within: Bounds, panel: Size) -> Room {
        let short = panel.width.min(panel.height);
        let (edge, gap) = (short * FINGER_EDGE, short * FINGER_GAP);
        let inside = (edge, edge, panel.width - edge, panel.height - edge);

        let centre = (
            f64::from(centre.0).clamp(inside.0 + gap, inside.2 - gap),
            f64::from(centre.1).clamp(inside.1 + gap, inside.3 - gap),
        );

        let area = (
            f64::from(within.x1).max(inside.0),
            f64::from(within.y1).max(inside.1),
            f64::from(within.x2).min(inside.2),
            f64::from(within.y2).min(inside.3),
        );
        let area = match area.0 < area.2 && area.1 < area.3 {
            true => area,
            false => inside,
        };

        Room {
            centre,
            area,
            inside,
            gap,
        }
    }

    fn reach(&self, (x1, y1, x2, y2): Rect, (dx, dy): (f64, f64)) -> f64 {
        let (cx, cy) = self.centre;
        let axis = |d: f64, c: f64, lo: f64, hi: f64| match d.abs() < 1e-9 {
            true => f64::INFINITY,
            false => (c - lo).min(hi - c).max(0.0) / d.abs(),
        };

        axis(dx, cx, x1, x2).min(axis(dy, cy, y1, y2))
    }

    fn half(&self, reach: impl Fn(Rect) -> f64) -> f64 {
        reach(self.area).max(self.gap).min(reach(self.inside))
    }

    fn still(&self, span: [f64; 2], turn: [f64; 2]) -> Gesture {
        Gesture {
            centre: [self.centre; 2],
            span,
            turn,
            fingers: 2,
            taps: 1,
        }
    }
}

pub fn fingers(centre: (i32, i32), within: Bounds, panel: Size, factor: f64, angle: f64) -> Result<Gesture> {
    if !factor.is_finite() || factor <= 0.0 || (factor - 1.0).abs() < 0.01 {
        bail!("a pinch factor is above 1 to spread the fingers or below 1 to close them, not {factor}");
    }

    let room = Room::new(centre, within, panel);
    let line = (angle.to_radians().cos(), angle.to_radians().sin());
    let wide = 2.0 * room.half(|r| room.reach(r, line));
    let narrow = (wide / factor.max(1.0 / factor)).max(room.gap);

    if narrow >= wide {
        bail!("the panel has no room for two fingers to pinch at {},{}", centre.0, centre.1);
    }

    let span = match factor > 1.0 {
        true => [narrow, wide],
        false => [wide, narrow],
    };

    Ok(room.still(span, [angle; 2]))
}

pub fn twist(centre: (i32, i32), within: Bounds, panel: Size, degrees: f64) -> Result<Gesture> {
    if !degrees.is_finite() || !(1.0..=360.0).contains(&degrees.abs()) {
        bail!("a rotation is between 1 and 360 degrees either way, not {degrees}");
    }

    let room = Room::new(centre, within, panel);
    let radius = room.half(|r| room.reach(r, (1.0, 0.0)).min(room.reach(r, (0.0, 1.0))));

    Ok(room.still([2.0 * radius; 2], [0.0, degrees]))
}

pub fn side_by_side(from: (i32, i32), to: (i32, i32), panel: Size, fingers: usize) -> Gesture {
    let short = panel.width.min(panel.height);
    let span = short * FINGER_SPACING * fingers.saturating_sub(1) as f64;
    let (dx, dy) = (f64::from(to.0 - from.0), f64::from(to.1 - from.1));
    let turn = match from == to {
        true => 0.0,
        false => dy.atan2(dx).to_degrees() + 90.0,
    };

    let edge = short * FINGER_EDGE;
    let reach = (
        span / 2.0 * turn.to_radians().cos().abs() + edge,
        span / 2.0 * turn.to_radians().sin().abs() + edge,
    );
    let fit = |(x, y): (i32, i32)| {
        (
            f64::from(x).clamp(reach.0, (panel.width - reach.0).max(reach.0)),
            f64::from(y).clamp(reach.1, (panel.height - reach.1).max(reach.1)),
        )
    };

    Gesture {
        centre: [fit(from), fit(to)],
        span: [span; 2],
        turn: [turn; 2],
        fingers,
        taps: 1,
    }
}

pub async fn gesture(t: &Target, g: &Gesture, over: Duration) -> Result<()> {
    let frames = g.frames(over);
    let step = over.as_millis() as usize / (frames.len() - 1);
    let a = match t {
        Target::Adb(a) => a,
        Target::Simulator(s) => return simctl::gesture(&s.at, &s.udid, g, &frames, step).await,
    };

    let Some(reader) = reader() else {
        bail!("this phone was built without its on-device helper (PHONE_DUMP_DEX), which a gesture needs");
    };

    let points: Vec<String> = frames
        .iter()
        .flatten()
        .flat_map(|(x, y)| [x.to_string(), y.to_string()])
        .collect();
    let args = format!(
        "touch {} {step} {} {} {} {} 2>&1",
        a.display.map_or(0, |d| d.logical),
        g.fingers,
        g.taps,
        TAP_GAP.as_millis(),
        points.join(" ")
    );

    let limit = SHELL_TIMEOUT + (over + TAP_GAP) * g.taps as u32;
    let (_, bytes) = tokio::time::timeout(
        limit,
        on_helper(a, &reader, |prefix| format!("{prefix}{}", launch(&reader, &args))),
    )
    .await
    .map_err(|_| anyhow!("the gesture did not finish within {}s", limit.as_secs()))??;

    let said = String::from_utf8_lossy(&bytes);

    if !said.contains(TOUCHED) {
        bail!("the device refused the gesture: {}", said.trim());
    }

    Ok(())
}

/// What `input text` cannot carry. It spells characters through the device
/// KeyCharacterMap, which covers printable ASCII: the rest is dropped on the way
/// and the command still exits 0.
fn unsendable(text: &str) -> Vec<char> {
    let mut odd: Vec<char> = text
        .chars()
        .filter(|c| !c.is_ascii_graphic() && *c != ' ')
        .collect();

    odd.sort_unstable();
    odd.dedup();

    odd
}

pub fn sendable(text: &str) -> Result<()> {
    let odd = unsendable(text);

    if !odd.is_empty() {
        bail!("cannot type {odd:?} — the device spells out ASCII only and drops the rest silently");
    }

    Ok(())
}

pub async fn type_text(t: &Target, text: &str) -> Result<()> {
    sendable(text)?;

    match t {
        Target::Adb(a) => input(a, &format!("text {}", shell_quote(text))).await,
        Target::Simulator(s) => simctl::type_text(&s.at, &s.udid, text).await,
    }
}

/// `input keyevent` exits 0 on a name it does not know and prints nothing, so
/// names are checked here. A bare number reaches the codes not listed.
/// HIDE_KEYBOARD is no keycode: it is BACK, sent only while the keyboard is up.
const KEYS: &str = "APP_SWITCH BACK CALL CAMERA DEL DPAD_CENTER DPAD_DOWN DPAD_LEFT DPAD_RIGHT \
     DPAD_UP ENDCALL ENTER ESCAPE FORWARD_DEL HIDE_KEYBOARD HOME MEDIA_NEXT MEDIA_PLAY_PAUSE \
     MEDIA_PREVIOUS MENU MOVE_END MOVE_HOME NOTIFICATION PAGE_DOWN PAGE_UP POWER SEARCH SETTINGS \
     SLEEP TAB VOLUME_DOWN VOLUME_MUTE VOLUME_UP WAKEUP";

const HIDE_KEYBOARD: &str = "HIDE_KEYBOARD";

fn keycode(name: &str) -> Result<String> {
    let name = name.trim().to_uppercase().replace('-', "_");
    let name = name.strip_prefix("KEYCODE_").unwrap_or(&name);

    if name.parse::<u16>().is_ok() || KEYS.split_whitespace().any(|key| key == name) {
        return Ok(name.to_string());
    }

    let near: Vec<&str> = KEYS
        .split_whitespace()
        .filter(|key| key.contains(name) || name.contains(key))
        .collect();

    if near.is_empty() {
        bail!("unknown key '{name}' (try: back, home, enter, tab)")
    }

    bail!("unknown key '{name}' — did you mean {}?", near.join(", "))
}

pub async fn key(t: &Target, name: &str) -> Result<String> {
    // Both sides are addressed by the Android key name, so `phone key home` is
    // one command whatever answers it. Validated against the list each backend
    // actually has rather than against a union of both.
    let a = match t {
        Target::Adb(a) => a,
        Target::Simulator(_) if name.eq_ignore_ascii_case("back") => return crate::scroll::back(t).await,
        Target::Simulator(s) => {
            simctl::key(&s.at, &s.udid, name).await?;

            return Ok(format!("sent {}", name.to_uppercase()));
        }
    };

    let code = keycode(name)?;

    if code == HIDE_KEYBOARD {
        return Ok(match hide_keyboard(a).await? {
            true => "closed the keyboard".to_string(),
            false => "the keyboard was already down".to_string(),
        });
    }

    if code == "BACK" && keyboard(a).await?.shown {
        hide_keyboard(a).await?;

        return Ok("sent BACK, which closed the keyboard".to_string());
    }

    input(a, &format!("keyevent {code}")).await?;

    Ok(format!("sent {code}"))
}

pub async fn keyboard(a: &Adb) -> Result<Keyboard> {
    let out = adb::run_timeout(
        &a.server,
        &["-s", &a.serial, "shell", KEYBOARD],
        Duration::from_secs(20),
    )
    .await?;

    Keyboard::parse(&out.stdout)
        .ok_or_else(|| anyhow!("dumpsys input_method does not say whether the keyboard is up"))
}

const KEYBOARD_GONE: Duration = Duration::from_secs(3);

async fn hide_keyboard(a: &Adb) -> Result<bool> {
    if !keyboard(a).await?.shown {
        return Ok(false);
    }

    input(a, "keyevent BACK").await?;

    let started = std::time::Instant::now();

    while started.elapsed() < KEYBOARD_GONE {
        tokio::time::sleep(Duration::from_millis(100)).await;

        if !keyboard(a).await?.shown {
            return Ok(true);
        }
    }

    bail!(
        "the keyboard was still up {}s after BACK",
        KEYBOARD_GONE.as_secs()
    )
}

/// Below this API level select-all cannot be sent from a shell, so a field is
/// emptied a character at a time.
const KEYCOMBINATION_SDK: u32 = 33;

pub async fn clear(t: &Target, len: usize) -> Result<()> {
    let Target::Adb(a) = t else {
        bail!("clearing a field needs Android");
    };

    shell(a, &clear_script(&a.input(), len)).await
}

fn clear_script(input: &str, len: usize) -> String {
    let dels = vec!["DEL"; len.max(1)].join(" ");

    format!(
        "if [ \"$(getprop ro.build.version.sdk)\" -ge {KEYCOMBINATION_SDK} ]; then \
         {input} keycombination CTRL_LEFT A && {input} keyevent DEL; \
         else {input} keyevent MOVE_END {dels}; fi"
    )
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
 <node class="android.widget.FrameLayout" bounds="[0,0][1080,2400]" clickable="false" text="" content-desc="" resource-id="">
  <node class="android.widget.TextView" bounds="[40,100][600,180]" clickable="false" text="Sign in" content-desc="" resource-id="com.app:id/title"/>
  <node class="android.widget.EditText" bounds="[40,300][1040,400]" clickable="true" text="" content-desc="Email" resource-id="com.app:id/email"/>
  <node class="android.widget.Button" bounds="[40,500][1040,600]" clickable="true" text="Log in" content-desc="" resource-id="com.app:id/submit"/>
  <node class="android.widget.Button" bounds="[0,0][0,0]" clickable="true" text="Collapsed" content-desc="" resource-id=""/>
  <node class="android.view.View" bounds="[40,700][1040,800]" clickable="true" text="" content-desc="" resource-id=""/>
 </node>
</hierarchy>"#;

    #[test]
    fn every_window_is_read_but_the_bars_and_the_keyboard() {
        let xml = r#"<displays><display id="0">
<window index="0" bounds="[0,2037][686,2352]" focused="false" type="TYPE_SYSTEM"><hierarchy rotation="0">
 <node class="android.widget.TextView" bounds="[40,2100][600,2200]" clickable="false" text="6TEAB" content-desc="" resource-id=""/>
</hierarchy></window>
<window index="1" bounds="[0,0][1080,136]" focused="false" type="TYPE_SYSTEM"><hierarchy rotation="0">
 <node class="android.widget.TextView" bounds="[40,20][200,120]" clickable="false" text="4:02" content-desc="" resource-id=""/>
</hierarchy></window>
<window index="2" bounds="[0,1500][1080,2400]" focused="false" type="TYPE_INPUT_METHOD"><hierarchy rotation="0">
 <node class="android.view.View" bounds="[0,1600][100,1700]" clickable="true" text="q" content-desc="" resource-id=""/>
</hierarchy></window>
<window index="3" title="Repbit" bounds="[0,0][1080,2400]" focused="true" type="TYPE_APPLICATION"><hierarchy rotation="0">
 <node class="android.widget.Button" bounds="[40,500][1040,600]" clickable="true" text="EMPEZAR" content-desc="" resource-id=""/>
</hierarchy></window>
</display></displays>"#;

        let nodes = parse(xml).unwrap();
        let labels: Vec<String> = nodes.iter().map(|n| n.label()).collect();

        assert_eq!(labels, ["6TEAB", "EMPEZAR"]);
        assert!(
            nodes[1].ancestors.is_empty(),
            "a window is not a box around its rows"
        );
    }

    #[test]
    fn keeps_only_what_can_be_read_or_pressed() {
        let nodes = parse(SAMPLE).expect("sample must parse");

        let labels: Vec<String> = nodes.iter().map(|n| n.label()).collect();
        assert_eq!(labels, ["Sign in", "Email", "Log in", "<View>"]);
    }

    /// The class fallback is a description of a row, not a handle on it. It
    /// used to print bare, which reads as something to press, and a screen of
    /// React Native containers prints seven of them.
    #[test]
    fn a_nameless_element_prints_as_its_class_and_answers_to_nothing() {
        let nodes = parse(SAMPLE).unwrap();
        let view = nodes.last().unwrap();

        assert_eq!(view.label(), "<View>");
        assert_eq!(view.name(), "");
        assert!(!view.matches("View"), "the class is not a name");
        assert!(
            pick(&nodes, "View").is_err(),
            "a printed <View> must not resolve"
        );
        assert_eq!(pick(&nodes, "@3").unwrap().bounds.y1, 700);
    }

    /// A crop of a label is a crop of the words on a card, not of the card.
    /// The boxes it sits in are what `--expand` widens to, and the ones that
    /// share its rectangle have to be skipped or `--expand 1` is a no-op.
    #[test]
    fn an_element_carries_the_boxes_it_sits_inside() {
        let xml = SAMPLE.replace(
            r#"<node class="android.widget.TextView" bounds="[40,100][600,180]" clickable="false" text="Sign in" content-desc="" resource-id="com.app:id/title"/>"#,
            r#"<node class="android.view.ViewGroup" bounds="[20,80][620,200]" clickable="false" text="" content-desc="" resource-id="">
    <node class="android.view.View" bounds="[40,100][600,180]" clickable="false" text="" content-desc="" resource-id="">
     <node class="android.widget.TextView" bounds="[40,100][600,180]" clickable="false" text="Sign in" content-desc="" resource-id="com.app:id/title"/>
    </node>
   </node>"#,
        );

        let nodes = parse(&xml).unwrap();
        let title = pick(&nodes, "Sign in").unwrap();

        assert_eq!(
            title.enclosing(1),
            Bounds::parse("[20,80][620,200]"),
            "the wrapper drawn on the label's own rectangle is not a box around it"
        );
        assert_eq!(title.enclosing(2), Bounds::parse("[0,0][1080,2400]"));
        assert_eq!(title.enclosures(), 2);
        assert_eq!(title.enclosing(3), None);
    }

    /// The root has nothing above it, and a caller asking to widen from there
    /// needs that told apart from a snapshot that carries no boxes at all.
    #[test]
    fn an_element_at_the_top_encloses_nothing() {
        let nodes = parse(SAMPLE).unwrap();

        assert_eq!(nodes[0].enclosures(), 1, "only the frame is above it");
        assert!(nodes[0].enclosing(2).is_none());
    }

    #[test]
    fn strips_the_package_prefix_from_a_resource_id() {
        let nodes = parse(SAMPLE).unwrap();

        assert_eq!(nodes[0].res_id, "title");
    }

    #[test]
    fn aims_at_the_middle_of_an_element() {
        let nodes = parse(SAMPLE).unwrap();
        let button = pick(&nodes, "Log in").unwrap();

        assert_eq!(button.bounds.center(), (540, 550));
    }

    #[test]
    fn refuses_an_ambiguous_needle() {
        let nodes = parse(SAMPLE).unwrap();
        let err = pick(&nodes, "i").unwrap_err().to_string();

        assert!(err.contains("only part of"), "{err}");
        assert!(
            err.contains("@0"),
            "the alternatives must be addressable: {err}"
        );
    }

    #[test]
    fn a_name_that_is_only_part_of_one_element_is_refused() {
        let xml = SAMPLE.replace(r#"text="Log in""#, r#"text="Log in with Google""#);
        let err = pick(&parse(&xml).unwrap(), "Log in").unwrap_err();

        assert!(err.is::<Ambiguous>(), "{err}");
        assert!(err.to_string().contains("@"), "{err}");
    }

    #[test]
    fn an_exact_label_beats_the_substring_matches_around_it() {
        let xml = SAMPLE.replace(r#"text="Sign in""#, r#"text="Log in now""#);
        let nodes = parse(&xml).unwrap();

        assert_eq!(pick(&nodes, "Log in").unwrap().res_id, "submit");
    }

    #[test]
    fn addresses_an_element_by_its_index() {
        let nodes = parse(SAMPLE).unwrap();

        assert_eq!(pick(&nodes, "@1").unwrap().desc, "Email");
        assert!(pick(&nodes, "@9").is_err());
    }

    const PANEL: Size = Size {
        width: 1080.0,
        height: 2400.0,
        scale: 1.0,
    };

    #[test]
    fn a_swipe_runs_through_the_middle_and_stops_short_of_the_edges() {
        let (from, to) = along(PANEL, Direction::Up, 0.6);

        assert_eq!(from, (540, 1920), "the finger starts low and travels up");
        assert_eq!(to, (540, 480));
        assert!(to.1 > 0, "an edge-to-edge drag is a system gesture");

        let (from, to) = along(PANEL, Direction::Down, 0.6);
        assert_eq!((from, to), ((540, 480), (540, 1920)));

        let (from, to) = along(PANEL, Direction::Left, 0.5);
        assert_eq!((from, to), ((810, 1200), (270, 1200)));
    }

    #[test]
    fn an_absurd_amount_is_clamped_rather_than_refused() {
        let (from, to) = along(PANEL, Direction::Up, 4.0);

        assert!(from.1 <= 2400 && to.1 >= 0, "{from:?} to {to:?}");
    }

    /// A simulator reports bounds in points and screenshots at 3x, so a crop
    /// taken from bounds unscaled lands in the top-left ninth of the frame.
    #[test]
    fn element_bounds_are_found_in_the_frame_through_the_scale() {
        let ios = Size {
            width: 440.0,
            height: 956.0,
            scale: 3.0,
        };

        let button = Bounds {
            x1: 20,
            y1: 100,
            x2: 420,
            y2: 148,
        };

        assert_eq!(
            ios.in_pixels(button),
            Bounds {
                x1: 60,
                y1: 300,
                x2: 1260,
                y2: 444
            }
        );
        assert_eq!(PANEL.in_pixels(button), button, "android reports pixels");
    }

    #[test]
    fn padding_an_element_stays_on_the_panel() {
        let edge = Bounds {
            x1: 0,
            y1: 10,
            x2: 1080,
            y2: 90,
        };

        assert_eq!(
            edge.padded(24, Some((1080, 2400))),
            Bounds {
                x1: 0,
                y1: 0,
                x2: 1080,
                y2: 114
            }
        );
    }

    #[test]
    fn presence_does_not_care_how_many_things_match() {
        let nodes = parse(SAMPLE).unwrap();

        assert!(present(&nodes, "i"), "ambiguous is still present");
        assert!(!present(&nodes, "Log out"));

        assert!(
            !present(&nodes, "@2"),
            "an index is a row number in one dump, not a name a later dump can answer to"
        );
    }

    #[test]
    fn refuses_a_key_the_device_would_silently_drop() {
        assert_eq!(keycode("back").unwrap(), "BACK");
        assert_eq!(keycode("KEYCODE_HOME").unwrap(), "HOME");
        assert_eq!(keycode("66").unwrap(), "66", "a raw code reaches the rest");

        let err = keycode("voluem_up").unwrap_err().to_string();
        assert!(err.contains("unknown key"), "{err}");
    }

    #[test]
    fn points_at_the_key_that_was_probably_meant() {
        let err = keycode("volume").unwrap_err().to_string();

        assert!(
            err.contains("VOLUME_UP") && err.contains("VOLUME_DOWN"),
            "{err}"
        );
    }

    #[test]
    fn quotes_text_so_the_device_shell_sees_one_word() {
        assert_eq!(shell_quote("two words"), "'two words'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn spanish_text_is_refused_rather_than_half_typed() {
        assert!(unsendable("hola que tal").is_empty());
        assert_eq!(unsendable("Menú de opciónes"), vec!['ó', 'ú']);
        assert_eq!(unsendable("line\nbreak"), vec!['\n']);
    }

    const FORM: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
 <node class="android.widget.FrameLayout" bounds="[0,0][1080,2400]" clickable="false" text="" content-desc="" resource-id="">
  <node class="android.widget.EditText" bounds="[40,300][1040,400]" clickable="true" focused="true" text="Search settings" hint="Search settings" content-desc="" resource-id="com.app:id/search"/>
  <node class="android.widget.EditText" bounds="[40,500][1040,600]" clickable="true" focused="false" text="hunter2" hint="" password="true" content-desc="" resource-id="com.app:id/secret"/>
  <node class="android.view.ViewGroup" bounds="[40,1800][1040,1950]" clickable="true" text="" content-desc="" resource-id="com.app:id/row">
   <node class="android.widget.TextView" bounds="[80,1820][600,1930]" clickable="false" text="Continue" content-desc="" resource-id=""/>
  </node>
 </node>
</hierarchy>"#;

    const IME_UP: &str = "  mInputShown=true\n  Window #3 Window{f00 u0 InputMethod}: ty=INPUT_METHOD type=ime frame=[0,1500][1080,2400] visibleFrame=[0,1500][1080,2400] visible=true\n";

    const IME_DOWN: &str = "  mInputShown=false\n  Window #3 Window{f00 u0 InputMethod}: ty=INPUT_METHOD type=ime frame=[0,0][0,0] visibleFrame=[0,0][0,0] visible=false\n";

    const DISPLAYS: &str = "  Display: mDisplayId=0 (organized)\n    init=2076x2152 420dpi cur=2152x2076 app=2152x1950 rng=1840x1840-2152x2152\n  Display: mDisplayId=3\n    init=1080x2424 420dpi cur=1080x2424 app=1080x2300 rng=1080x1080-2424x2424\n";

    const PANEL_FOLD: Bounds = Bounds {
        x1: 0,
        y1: 0,
        x2: 2076,
        y2: 2152,
    };

    #[test]
    fn the_panel_is_read_for_the_display_being_driven() {
        let rotated = Bounds::parse("[0,0][2152,2076]");

        assert_eq!(panel_of(DISPLAYS, 0), rotated);
        assert_eq!(panel_of(DISPLAYS, 3), Bounds::parse("[0,0][1080,2424]"));
        assert_eq!(panel_of(DISPLAYS, 1), None);
        assert_eq!(panel_of("", 0), None);

        let screen = read_dump(true, 0, &format!("{IME_DOWN}{DISPLAYS}{FORM}")).unwrap();
        assert_eq!(screen.panel, rotated);
    }

    #[test]
    fn a_window_stashed_off_the_left_edge_is_aimed_at_its_visible_strip() {
        let pip = Bounds::parse("[-656,200][78,613]").unwrap();

        assert_eq!(
            Reach::of(pip, Some(PANEL_FOLD)),
            Reach::Partly {
                at: (39, 406),
                edge: "left",
                showing: 78,
            }
        );
        assert_eq!(
            Reach::of(pip, Some(PANEL_FOLD)).note().as_deref(),
            Some("mostly off the left edge, 78px showing")
        );
        assert_eq!(Reach::of(pip, None), Reach::On((-289, 406)));
    }

    #[test]
    fn an_element_with_its_centre_on_the_panel_keeps_its_centre() {
        let half = Bounds::parse("[-100,500][300,600]").unwrap();

        assert_eq!(Reach::of(half, Some(PANEL_FOLD)), Reach::On((100, 550)));
        assert_eq!(Reach::On((100, 550)).note(), None);
    }

    #[test]
    fn the_edge_named_is_the_one_most_of_the_element_is_past() {
        let low = Bounds::parse("[1900,2100][2300,2900]").unwrap();

        assert_eq!(
            Reach::of(low, Some(PANEL_FOLD)),
            Reach::Partly {
                at: (1988, 2126),
                edge: "bottom",
                showing: 52,
            }
        );
    }

    #[test]
    fn an_element_wholly_off_the_panel_offers_no_point() {
        let gone = Bounds::parse("[-900,200][-10,600]").unwrap();
        let reach = Reach::of(gone, Some(PANEL_FOLD));

        assert_eq!(reach, Reach::Off);
        assert_eq!(reach.point(), None);
        assert_eq!(reach.note().as_deref(), Some("off the panel"));
    }

    #[test]
    fn the_keyboard_is_judged_at_the_point_that_would_be_pressed() {
        let nodes = parse(
            r#"<hierarchy><node class="android.widget.Button" bounds="[-656,1600][78,1700]" clickable="true" text="Pip" content-desc="" resource-id=""/></hierarchy>"#,
        )
        .unwrap();
        let mut screen = Screen {
            nodes,
            keyboard: Some(Keyboard {
                shown: true,
                frame: Bounds::parse("[0,1500][1080,2400]"),
            }),
            panel: None,
        };

        assert!(!screen.covered(&screen.nodes[0]), "the centre is off the keyboard");

        screen.panel = Some(PANEL_FOLD);
        assert!(screen.covered(&screen.nodes[0]), "the strip that shows is under it");
    }

    #[test]
    fn a_screen_that_never_goes_idle_is_told_apart_from_one_that_is_off() {
        let err = read_dump(true, 0, "phone:not-idle\n").unwrap_err();

        assert!(err.is::<NotIdle>(), "{err}");
        assert!(err.to_string().contains("Remove animations"), "{err}");

        let err = read_dump(true, 0, "  mInputShown=false\n").unwrap_err();
        assert!(!err.is::<NotIdle>(), "{err}");
        assert!(err.to_string().contains("no hierarchy"), "{err}");
    }

    #[test]
    fn the_scales_are_read_off_a_screen_that_never_went_idle() {
        let err = read_dump(true, 0, "phone:not-idle\nphone:scales null 1.0 0\n").unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("transition_animation_scale is 1"), "{msg}");

        for garbled in [
            "phone:scales 1 1\n",
            "phone:scales 1 x 1\n",
            "phone:scales\n",
            "",
        ] {
            assert_eq!(Scales::parse(garbled), None, "{garbled:?}");
        }
    }

    #[test]
    fn the_keyboard_is_read_off_what_comes_before_the_hierarchy() {
        let up = read_dump(true, 0, &format!("{IME_UP}{FORM}")).unwrap();

        assert_eq!(
            up.keyboard,
            Some(Keyboard {
                shown: true,
                frame: Bounds::parse("[0,1500][1080,2400]"),
            })
        );
        assert_eq!(
            up.keyboard.unwrap().describe(),
            "up over [0,1500][1080,2400]"
        );

        let down = read_dump(true, 0, &format!("{IME_DOWN}{FORM}")).unwrap();

        assert_eq!(
            down.keyboard,
            Some(Keyboard {
                shown: false,
                frame: None
            })
        );
        assert_eq!(down.nodes.len(), up.nodes.len());

        let silent = read_dump(true, 0, FORM).unwrap();
        assert_eq!(silent.keyboard, None, "unknown is not down");
    }

    #[test]
    fn a_row_whose_middle_is_under_the_keyboard_is_covered() {
        let screen = read_dump(true, 0, &format!("{IME_UP}{FORM}")).unwrap();
        let covered: Vec<String> = screen
            .nodes
            .iter()
            .filter(|n| screen.covered(n))
            .map(|n| n.label())
            .collect();

        assert_eq!(covered, ["row", "Continue"]);

        let down = read_dump(true, 0, &format!("{IME_DOWN}{FORM}")).unwrap();
        assert!(down.nodes.iter().all(|n| !down.covered(n)));
    }

    #[test]
    fn a_field_showing_its_hint_is_empty() {
        let screen = read_dump(true, 0, FORM).unwrap();
        let search = screen.focused().expect("the search field holds focus");

        assert_eq!(search.res_id, "search");
        assert!(search.is_empty_field());
        assert!(search.reads(""));
        assert!(
            !search.reads("Search settings"),
            "asking for the hint's words is asking for text"
        );
        assert_eq!(
            search.describe_field(),
            r#"search (EditText) empty, hint "Search settings""#
        );

        let secret = pick(&screen.nodes, "secret").unwrap();
        assert!(secret.password);
        assert_eq!(secret.describe_field(), "secret (EditText) password");
    }

    #[test]
    fn a_focused_field_is_kept_even_with_nothing_to_name_it() {
        let xml = FORM.replace(
            r#"text="Search settings" hint="Search settings" content-desc="" resource-id="com.app:id/search""#,
            r#"text="" hint="" content-desc="" resource-id="""#,
        )
        .replace(r#"clickable="true" focused="true""#, r#"clickable="false" focused="true""#);

        let screen = read_dump(true, 0, &xml).unwrap();

        assert_eq!(screen.focused().unwrap().label(), "<EditText>");
    }

    #[test]
    fn a_name_on_a_pressable_and_the_label_inside_it_picks_the_pressable() {
        let xml = FORM.replace(
            r#"text="" content-desc="" resource-id="com.app:id/row""#,
            r#"text="" content-desc="Continue" resource-id="com.app:id/row""#,
        );
        let nodes = parse(&xml).unwrap();

        assert_eq!(pick(&nodes, "Continue").unwrap().res_id, "row");
    }

    #[test]
    fn a_name_on_one_pressable_and_a_loose_label_picks_the_pressable() {
        let xml = FORM.replace(
            r#"  <node class="android.widget.EditText" bounds="[40,500]"#,
            r#"  <node class="android.widget.TextView" bounds="[40,1000][1040,1100]" clickable="false" text="Search settings" content-desc="" resource-id=""/>
  <node class="android.widget.EditText" bounds="[40,500]"#,
        );

        assert_eq!(
            pick(&parse(&xml).unwrap(), "Search settings")
                .unwrap()
                .res_id,
            "search"
        );
    }

    #[test]
    fn side_by_side_matches_are_ambiguous_and_no_match_is_missing() {
        let err = pick(&parse(FORM).unwrap(), "e").unwrap_err();

        assert!(err.is::<Ambiguous>(), "{err}");

        let err = pick(&parse(FORM).unwrap(), "Checkout").unwrap_err();

        assert!(err.is::<Missing>(), "{err}");
    }

    #[test]
    fn a_name_that_matches_nothing_offers_only_near_spellings() {
        let nodes = parse(&SAMPLE.replace(r#"text="Sign in""#, r#"text="Configuración""#)).unwrap();
        let err = |needle| pick(&nodes, needle).unwrap_err().to_string();

        assert_eq!(
            err("configuracion"),
            "nothing on screen matches 'configuracion'; close: 'Configuración'"
        );
        assert_eq!(
            err("Login"),
            "nothing on screen matches 'Login'; close: 'Log in'"
        );
        assert_eq!(err("Volver"), "nothing on screen matches 'Volver'");

        let steps = parse(&SAMPLE.replace(r#"text="Sign in""#, r#"text="Step 2""#)).unwrap();
        let err = pick(&steps, "Step 1").unwrap_err().to_string();

        assert_eq!(err, "nothing on screen matches 'Step 1'");
    }

    #[test]
    fn an_index_names_the_row_it_was_shown_on_not_the_position() {
        let before = parse(FORM).unwrap();
        let shown: Vec<Signature> = before.iter().map(Node::signature).collect();

        let xml = FORM.replace(
            r#"  <node class="android.widget.EditText" bounds="[40,300]"#,
            r#"  <node class="android.widget.TextView" bounds="[40,100][1040,200]" clickable="false" text="Offline" content-desc="" resource-id=""/>
  <node class="android.widget.EditText" bounds="[40,300]"#,
        );
        let after = parse(&xml).unwrap();

        assert_eq!(pick(&after, "@1").unwrap().res_id, "search", "positional");
        assert_eq!(
            pick_in(&after, "@1", Some(&shown)).unwrap().res_id,
            "secret"
        );
    }

    #[test]
    fn a_field_is_found_again_after_its_text_and_size_changed() {
        let shown: Vec<Signature> = parse(FORM).unwrap().iter().map(Node::signature).collect();
        let typed = parse(
            &FORM
                .replace(r#"text="Search settings" hint"#, r#"text="wifi" hint"#)
                .replace("[40,300][1040,400]", "[40,300][900,400]"),
        )
        .unwrap();

        assert_eq!(pick_in(&typed, "@0", Some(&shown)).unwrap().text, "wifi");
    }

    #[test]
    fn an_id_shared_by_two_rows_does_not_find_a_changed_one() {
        let shown: Vec<Signature> = parse(FORM).unwrap().iter().map(Node::signature).collect();
        let twice = parse(
            &FORM
                .replace(r#"text="Search settings" hint"#, r#"text="wifi" hint"#)
                .replace("com.app:id/secret", "com.app:id/search"),
        )
        .unwrap();

        assert!(pick_in(&twice, "@0", Some(&shown)).is_err());
    }

    #[test]
    fn an_index_whose_row_moved_is_refused() {
        let shown: Vec<Signature> = parse(FORM).unwrap().iter().map(Node::signature).collect();
        let scrolled = parse(&FORM.replace("[80,1820][600,1930]", "[80,1620][600,1730]")).unwrap();

        let err = pick_in(&scrolled, "@3", Some(&shown))
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "@3 (Continue) moved; it is still on screen, name it 'Continue' instead"
        );

        let twice = parse(
            &FORM
                .replace("[80,1820][600,1930]", "[80,1620][600,1730]")
                .replace(
                    r#"text="Continue" "#,
                    r#"text="Continue"/><node class="android.widget.TextView" bounds="[80,2000][600,2100]" text="Continue" "#,
                ),
        )
        .unwrap();

        let err = pick_in(&twice, "@3", Some(&shown)).unwrap_err().to_string();
        assert!(err.contains("@3 (Continue) is no longer where"), "{err}");

        let err = pick_in(&scrolled, "@7", Some(&shown))
            .unwrap_err()
            .to_string();
        assert!(err.contains("no element @7 in the last snapshot"), "{err}");
    }

    fn live(desc: &str) -> String {
        FORM.replace(
            r#"  <node class="android.view.ViewGroup" bounds="[40,1800]"#,
            &format!(
                r#"  <node class="android.view.ViewGroup" bounds="[40,700][1040,900]" clickable="true" text="" content-desc="{desc}" resource-id=""/>
  <node class="android.view.ViewGroup" bounds="[40,1800]"#
            ),
        )
    }

    #[test]
    fn an_id_less_row_is_found_again_after_its_numbers_ticked() {
        for (then, now) in [
            (
                "100.118.60.77:357&#10;luisnquin · OPENSSH_10.5 · 30 MS&#10;DETAILS",
                "100.118.60.77:357&#10;luisnquin · OPENSSH_10.5 · 31 MS&#10;DETAILS",
            ),
            ("III&#10;III&#10;48/78  62%", "III&#10;III&#10;49/78  63%"),
        ] {
            let shown: Vec<Signature> = parse(&live(then))
                .unwrap()
                .iter()
                .map(Node::signature)
                .collect();
            let after = parse(&live(now)).unwrap();

            assert_eq!(
                pick_in(&after, "@2", Some(&shown)).unwrap().desc,
                now.replace("&#10;", "\n")
            );
        }
    }

    #[test]
    fn an_id_less_row_whose_words_changed_is_refused_and_says_why() {
        let shown: Vec<Signature> = parse(&live("III&#10;48/78  62%"))
            .unwrap()
            .iter()
            .map(Node::signature)
            .collect();
        let after = parse(&live("III&#10;Done")).unwrap();

        let err = pick_in(&after, "@2", Some(&shown)).unwrap_err().to_string();
        assert!(err.contains("is no longer where"), "{err}");
        assert!(err.contains("testID"), "{err}");
    }

    #[test]
    fn two_id_less_rows_whose_numbers_ticked_alike_are_refused() {
        let shown: Vec<Signature> = parse(&live("48/78  62%"))
            .unwrap()
            .iter()
            .map(Node::signature)
            .collect();
        let after = parse(&live("49/78  63%").replace(
            r#"content-desc="49/78  63%" resource-id=""/>"#,
            r#"content-desc="49/78  63%" resource-id=""/>
  <node class="android.view.ViewGroup" bounds="[40,700][1040,900]" clickable="true" text="" content-desc="50/78  64%" resource-id=""/>"#,
        ))
        .unwrap();

        assert!(pick_in(&after, "@2", Some(&shown)).is_err());
    }

    #[test]
    fn hiding_the_keyboard_is_a_key_by_either_spelling() {
        assert_eq!(keycode("hide_keyboard").unwrap(), HIDE_KEYBOARD);
        assert_eq!(keycode("hide-keyboard").unwrap(), HIDE_KEYBOARD);
    }

    #[test]
    fn a_field_is_emptied_by_select_all_where_the_device_can_send_it() {
        let script = clear_script("input -d 2", 3);

        assert!(
            script.contains("input -d 2 keycombination CTRL_LEFT A && input -d 2 keyevent DEL"),
            "{script}"
        );
        assert!(script.contains("keyevent MOVE_END DEL DEL DEL"), "{script}");
    }

    #[test]
    fn a_held_drag_stays_down_before_it_moves_and_lifts_at_the_end() {
        let script = drag_script(
            "input -d 2",
            (100, 1000),
            (100, 400),
            Duration::from_millis(1500),
            Duration::from_millis(300),
        );

        assert_eq!(
            script,
            "input -d 2 motionevent DOWN 100 1000; sleep 1.500; \
             input -d 2 motionevent MOVE 100 800; sleep 0.100; \
             input -d 2 motionevent MOVE 100 600; sleep 0.100; \
             input -d 2 motionevent MOVE 100 400; sleep 0.100; \
             input -d 2 motionevent UP 100 400"
        );
    }

    #[test]
    fn a_long_held_drag_moves_in_a_bounded_number_of_steps() {
        let script = drag_script(
            "input",
            (0, 0),
            (1000, 0),
            Duration::from_secs(1),
            Duration::from_secs(10),
        );

        assert_eq!(script.matches("MOVE").count(), 10);
        assert!(script.contains("MOVE 1000 0; sleep 1.000; input motionevent UP 1000 0"));
    }

    const PIXEL: Size = Size {
        width: 1080.0,
        height: 2400.0,
        scale: 1.0,
    };
    const PHOTO: Bounds = Bounds {
        x1: 440,
        y1: 1100,
        x2: 640,
        y2: 1300,
    };

    #[test]
    fn two_fingers_spread_or_close_inside_what_they_aim_at() {
        let (pixel, photo) = (PIXEL, PHOTO);
        let simulator = Size {
            width: 402.0,
            height: 874.0,
            scale: 3.0,
        };
        let whole = |s: Size| Bounds {
            x1: 0,
            y1: 0,
            x2: s.width as i32,
            y2: s.height as i32,
        };
        let icon = Bounds {
            x1: 530,
            y1: 1190,
            x2: 550,
            y2: 1210,
        };

        type Fingers = [(i32, i32); 2];
        type Case = (&'static str, Size, (i32, i32), Bounds, f64, f64, Fingers, Fingers);
        let cases: [Case; 9] = [
            (
                "out across the panel, inside its edges",
                pixel,
                (540, 1200),
                whole(pixel),
                2.0,
                0.0,
                [(297, 1200), (783, 1200)],
                [(54, 1200), (1026, 1200)],
            ),
            (
                "in is the same path walked backwards",
                pixel,
                (540, 1200),
                whole(pixel),
                0.5,
                0.0,
                [(54, 1200), (1026, 1200)],
                [(297, 1200), (783, 1200)],
            ),
            (
                "an element bounds the spread",
                pixel,
                (540, 1200),
                photo,
                2.0,
                0.0,
                [(490, 1200), (590, 1200)],
                [(440, 1200), (640, 1200)],
            ),
            (
                "a large factor stops at the closest two fingers come",
                pixel,
                (540, 1200),
                whole(pixel),
                100.0,
                0.0,
                [(497, 1200), (583, 1200)],
                [(54, 1200), (1026, 1200)],
            ),
            (
                "an element too small for two fingers lends them the panel",
                pixel,
                (540, 1200),
                icon,
                2.0,
                0.0,
                [(497, 1200), (583, 1200)],
                [(454, 1200), (626, 1200)],
            ),
            (
                "a point by the edge moves in until both fingers fit",
                pixel,
                (100, 1200),
                whole(pixel),
                2.0,
                0.0,
                [(97, 1200), (184, 1200)],
                [(54, 1200), (227, 1200)],
            ),
            (
                "vertical",
                pixel,
                (540, 1200),
                whole(pixel),
                2.0,
                90.0,
                [(540, 627), (540, 1773)],
                [(540, 54), (540, 2346)],
            ),
            (
                "diagonal, bounded by the nearer edge",
                pixel,
                (540, 1200),
                whole(pixel),
                2.0,
                45.0,
                [(297, 957), (783, 1443)],
                [(54, 714), (1026, 1686)],
            ),
            (
                "a simulator in points",
                simulator,
                (201, 437),
                whole(simulator),
                2.0,
                0.0,
                [(111, 437), (291, 437)],
                [(20, 437), (382, 437)],
            ),
        ];

        for (why, size, centre, within, factor, angle, from, to) in cases {
            let pinch = fingers(centre, within, size, factor, angle).unwrap();

            assert_eq!((pinch.at(0.0), pinch.at(1.0)), (from.to_vec(), to.to_vec()), "{why}");
        }

        for factor in [1.0, 0.0, -2.0, f64::NAN, f64::INFINITY] {
            assert!(
                fingers((540, 1200), photo, pixel, factor, 0.0).is_err(),
                "{factor} is no pinch"
            );
        }
    }

    #[test]
    fn a_gesture_moves_every_finger_a_step_at_a_time() {
        let g = |centre, span, turn, fingers| Gesture {
            centre,
            span,
            turn,
            fingers,
            taps: 1,
        };
        let pinch = g([(500.0, 1200.0); 2], [200.0, 800.0], [0.0; 2], 2);

        type Case = (&'static str, Gesture, Duration, Vec<Vec<(i32, i32)>>);
        let cases: [Case; 5] = [
            (
                "two fingers spread",
                pinch,
                Duration::from_millis(32),
                vec![
                    vec![(400, 1200), (600, 1200)],
                    vec![(250, 1200), (750, 1200)],
                    vec![(100, 1200), (900, 1200)],
                ],
            ),
            ("too short still takes two steps", pinch, Duration::ZERO, pinch.frames(Duration::from_millis(32))),
            (
                "a turn walks the circle, not the chord",
                g([(500.0, 500.0); 2], [200.0; 2], [0.0, 180.0], 2),
                Duration::from_millis(32),
                vec![
                    vec![(400, 500), (600, 500)],
                    vec![(500, 400), (500, 600)],
                    vec![(600, 500), (400, 500)],
                ],
            ),
            (
                "three fingers side by side travel together",
                g([(540.0, 1600.0), (540.0, 800.0)], [200.0; 2], [0.0; 2], 3),
                Duration::from_millis(32),
                vec![
                    vec![(440, 1600), (540, 1600), (640, 1600)],
                    vec![(440, 1200), (540, 1200), (640, 1200)],
                    vec![(440, 800), (540, 800), (640, 800)],
                ],
            ),
            (
                "one finger stays where it is put",
                g([(10.0, 20.0); 2], [0.0; 2], [0.0; 2], 1),
                Duration::ZERO,
                vec![vec![(10, 20)]; 3],
            ),
        ];

        for (why, gesture, over, frames) in cases {
            assert_eq!(gesture.frames(over), frames, "{why}");
        }

        assert_eq!(pinch.frames(Duration::from_millis(400)).len(), 26);
        assert_eq!(pinch.frames(Duration::from_secs(60)).len(), GESTURE_STEPS_MAX + 1);
    }

    #[test]
    fn fingers_turn_inside_their_element_and_travel_side_by_side() {
        let panel = Bounds {
            x1: 0,
            y1: 0,
            x2: 1080,
            y2: 2400,
        };

        type Case = (&'static str, Gesture, Vec<(i32, i32)>, Vec<(i32, i32)>);
        let cases: [Case; 5] = [
            (
                "a quarter turn across the panel",
                twist((540, 1200), panel, PIXEL, 90.0).unwrap(),
                vec![(54, 1200), (1026, 1200)],
                vec![(540, 714), (540, 1686)],
            ),
            (
                "anticlockwise inside an element",
                twist((540, 1200), PHOTO, PIXEL, -90.0).unwrap(),
                vec![(440, 1200), (640, 1200)],
                vec![(540, 1300), (540, 1100)],
            ),
            (
                "an upward swipe lines the fingers up across it",
                side_by_side((540, 1680), (540, 720), PIXEL, 3),
                vec![(410, 1680), (540, 1680), (670, 1680)],
                vec![(410, 720), (540, 720), (670, 720)],
            ),
            (
                "a sideways swipe stacks them",
                side_by_side((200, 1200), (900, 1200), PIXEL, 2),
                vec![(200, 1135), (200, 1265)],
                vec![(900, 1135), (900, 1265)],
            ),
            (
                "a tap by the edge moves in until every finger fits",
                side_by_side((20, 1200), (20, 1200), PIXEL, 2),
                vec![(54, 1200), (184, 1200)],
                vec![(54, 1200), (184, 1200)],
            ),
        ];

        for (why, gesture, from, to) in cases {
            assert_eq!((gesture.at(0.0), gesture.at(1.0)), (from, to), "{why}");
        }

        for degrees in [0.0, 0.5, -400.0, f64::NAN] {
            assert!(twist((540, 1200), PHOTO, PIXEL, degrees).is_err(), "{degrees} is no turn");
        }
    }

    #[test]
    fn a_typeset_label_answers_to_the_plain_spelling() {
        let nodes = parse(
            "<hierarchy><node text=\"Wi\u{2011}Fi\" class=\"android.widget.TextView\" bounds=\"[0,0][10,10]\"/>\
             <node text=\"Don\u{2019}t allow\" class=\"android.widget.Button\" bounds=\"[0,10][10,20]\"/></hierarchy>",
        )
        .unwrap();

        assert!(nodes[0].answers("wi-fi"));
        assert!(nodes[1].answers("Don't allow"));
        assert!(present(&nodes, "Don't"));
    }

    const CARDS: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
 <node class="android.widget.FrameLayout" bounds="[0,0][1080,2400]" clickable="false" text="" content-desc="" resource-id="">
  <node class="android.view.View" bounds="[0,330][1080,452]" clickable="true" text="" content-desc="Activa las notificaciones. Activar" resource-id="">
   <node class="android.widget.TextView" bounds="[40,340][800,440]" clickable="false" text="Activa las notificaciones." content-desc="" resource-id=""/>
   <node class="android.widget.TextView" bounds="[840,360][940,420]" clickable="false" text="Activar" content-desc="" resource-id=""/>
  </node>
  <node class="android.view.View" bounds="[800,160][900,260]" clickable="true" text="" content-desc="" resource-id="">
   <node class="android.view.View" bounds="[800,160][900,260]" clickable="false" text="" content-desc="Ver notificaciones" resource-id=""/>
  </node>
  <node class="android.view.View" bounds="[40,1700][520,2080]" clickable="true" text="" content-desc="" resource-id="com.app:id/newFixedFundCard">
   <node class="android.view.View" bounds="[40,1700][520,2080]" clickable="false" text="" content-desc="Con meta fija. Ideal para eventos." resource-id="">
    <node class="android.widget.TextView" bounds="[60,1720][500,1800]" clickable="false" text="Con meta fija" content-desc="" resource-id=""/>
    <node class="android.widget.TextView" bounds="[60,1900][500,2060]" clickable="false" text="Ideal para eventos." content-desc="" resource-id=""/>
   </node>
  </node>
  <node class="android.view.View" bounds="[560,1700][1040,2080]" clickable="true" text="" content-desc="" resource-id="">
   <node class="android.view.View" bounds="[560,1700][1040,2080]" clickable="false" text="" content-desc="Pozo libre. Sin límite." resource-id="">
    <node class="android.widget.TextView" bounds="[580,1720][1020,1800]" clickable="false" text="Pozo libre" content-desc="" resource-id=""/>
    <node class="android.widget.TextView" bounds="[580,1900][1020,2060]" clickable="false" text="Sin límite." content-desc="" resource-id=""/>
   </node>
  </node>
  <node class="android.view.View" bounds="[40,1000][1040,1100]" clickable="true" text="" content-desc="" resource-id="">
   <node class="android.widget.TextView" bounds="[60,1010][600,1090]" clickable="false" text="Olivia Owner" content-desc="" resource-id=""/>
   <node class="android.widget.Button" bounds="[800,1010][1020,1090]" clickable="true" text="Eliminar" content-desc="" resource-id=""/>
  </node>
  <node class="android.view.View" bounds="[0,0][1080,2400]" clickable="true" text="" content-desc="" resource-id=""/>
  <node class="android.widget.TextView" bounds="[300,1200][780,1300]" clickable="false" text="¿Seguro?" content-desc="" resource-id=""/>
 </node>
</hierarchy>"#;

    fn listed_rows(nodes: &[Node]) -> Vec<String> {
        rows(nodes)
            .iter()
            .filter(|r| r.within.is_none())
            .map(|r| format!("@{} {}", r.node.index, r.label))
            .collect()
    }

    #[test]
    fn a_label_its_row_already_reads_is_folded_into_it() {
        let nodes = parse(CARDS).unwrap();

        assert_eq!(
            listed_rows(&nodes),
            [
                "@0 Activa las notificaciones. Activar",
                "@3 Ver notificaciones",
                "@5 newFixedFundCard",
                "@6 Con meta fija. Ideal para eventos.",
                "@9 Pozo libre. Sin límite.",
                "@13 <View>",
                "@14 Olivia Owner",
                "@15 Eliminar",
                "@16 <View>",
                "@17 ¿Seguro?",
            ]
        );

        let within: Vec<Option<usize>> = rows(&nodes).iter().map(|r| r.within).collect();
        assert_eq!(within[11], Some(9), "folded through a folded row to the one listed");
        assert_eq!(within[7], Some(6));
    }

    #[test]
    fn a_folded_row_still_answers_to_its_name_and_index() {
        let nodes = parse(CARDS).unwrap();
        let bell = pick(&nodes, "Ver notificaciones").unwrap();

        assert!(nodes[3].bounds.holds(bell.bounds.center()));
        assert_eq!(pick(&nodes, "Activar").unwrap().index, 2);
        assert_eq!(pick(&nodes, "@12").unwrap().text, "Sin límite.");
        assert!(present(&nodes, "Con meta fija"));
    }

    #[test]
    fn a_label_cut_off_its_row_is_not_folded() {
        let long = "x".repeat(ROW_LIMIT);
        let xml = CARDS.replace(
            r#"content-desc="Activa las notificaciones. Activar""#,
            &format!(r#"content-desc="{long} Activar""#),
        );

        assert!(listed_rows(&parse(&xml).unwrap()).contains(&"@2 Activar".to_string()));
    }

    #[test]
    fn a_stack_trace_is_one_row() {
        let trace = "java.net.ConnectException: Failed\n at okhttp3.a\n at okhttp3.b";

        assert_eq!(row_label(trace), "java.net.ConnectException: Failed (+2 lines)");
        assert_eq!(row_label(&"x".repeat(150)), format!("{}…", "x".repeat(100)));
        assert_eq!(row_label("Inicio"), "Inicio");
    }
}
