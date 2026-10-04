use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Result};
use clap::ValueEnum;

use crate::a11y::{self, Bounds, Node, Screen, Size, Target};
use crate::answer::{self, How};
use crate::cli::{Command, Edge, DEFAULT_AMOUNT};
use crate::Session;

const SWIPE_DEFAULT: Duration = Duration::from_millis(300);
const SCROLL_AMOUNT: f64 = 0.4;
const SCROLL_DURATION: Duration = Duration::from_millis(500);
const NUDGE_DURATION: Duration = Duration::from_millis(900);
const NUDGES: usize = 2;
const BARS: f64 = 0.06;
const SLOP: f64 = 0.01;
const VIEWPORT: f64 = 1.5;

type Line = ((i32, i32), (i32, i32));

pub fn inward(size: Size, edge: Edge, amount: f64) -> Line {
    let amount = amount.clamp(0.05, 0.9);
    let (w, h) = (size.width - 1.0, size.height - 1.0);
    let at = |x: f64, y: f64| (x.round() as i32, y.round() as i32);

    match edge {
        Edge::Left => (at(0.0, h / 2.0), at(w * amount, h / 2.0)),
        Edge::Right => (at(w, h / 2.0), at(w * (1.0 - amount), h / 2.0)),
        Edge::Top => (at(w / 2.0, 0.0), at(w / 2.0, h * amount)),
        Edge::Bottom => (at(w / 2.0, h), at(w / 2.0, h * (1.0 - amount))),
    }
}

async fn swipe_in(t: &Target, edge: Edge, amount: f64, over: Duration) -> Result<String> {
    let (from, to) = inward(a11y::size(t).await?, edge, amount);

    a11y::swipe(t, from, to, over).await?;

    let side = edge
        .to_possible_value()
        .map(|v| v.get_name().to_string())
        .unwrap_or_default();

    Ok(format!(
        "swiped in from the {side} edge, {},{} to {},{} over {}ms",
        from.0,
        from.1,
        to.0,
        to.1,
        over.as_millis()
    ))
}

pub async fn edge(t: &Target, edge: Edge, amount: f64, over: Duration) -> Result<()> {
    let said = match (t, edge) {
        (Target::Simulator(_), Edge::Bottom) => format!(
            "{}: a simulator never hands a bottom-edge touch to the home indicator",
            a11y::key(t, "home").await?
        ),
        _ => swipe_in(t, edge, amount, over).await?,
    };

    eprintln!("phone: {said}");

    Ok(())
}

pub async fn back(t: &Target) -> Result<String> {
    let said = swipe_in(t, Edge::Left, DEFAULT_AMOUNT, SWIPE_DEFAULT).await?;

    Ok(format!("{said}: a simulator has no back key, and this is how iOS goes back"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn of((from, to): Line) -> Axis {
        match (to.0 - from.0).abs() > (to.1 - from.1).abs() {
            true => Axis::X,
            false => Axis::Y,
        }
    }

    fn span(self, b: Bounds) -> (i32, i32) {
        match self {
            Axis::X => (b.x1, b.x2),
            Axis::Y => (b.y1, b.y2),
        }
    }

    fn panel(self, size: Size) -> f64 {
        match self {
            Axis::X => size.width,
            Axis::Y => size.height,
        }
    }
}

struct Seen {
    keyboard: Option<Bounds>,
    size: Size,
    axis: Axis,
}

impl Seen {
    fn band(&self, along: Axis) -> (i32, i32) {
        let panel = along.panel(self.size);
        let (lo, hi) = match along {
            Axis::X => (0, panel as i32),
            Axis::Y => ((panel * BARS) as i32, (panel * (1.0 - BARS)) as i32),
        };

        match (along, self.keyboard) {
            (Axis::Y, Some(k)) => (lo, hi.min(k.y1)),
            _ => (lo, hi),
        }
    }

    fn room(&self, node: &Node, along: Axis) -> (i32, i32) {
        let (lo, hi) = self.band(along);
        let (n1, n2) = along.span(node.bounds);
        let viewport = node
            .ancestors
            .iter()
            .map(|b| along.span(*b))
            .filter(|_| along == self.axis)
            .find(|(v1, v2)| f64::from(v2 - v1) >= VIEWPORT * f64::from(n2 - n1));

        match viewport {
            Some((v1, v2)) => (lo.max(v1 + 1), hi.min(v2 - 1)),
            None => (lo, hi),
        }
    }

    fn clear(&self, node: &Node) -> bool {
        [Axis::X, Axis::Y].into_iter().all(|along| {
            let (lo, hi) = self.room(node, along);
            let (n1, n2) = along.span(node.bounds);

            lo <= n1 && n2 <= hi
        })
    }

    fn nudge(&self, node: &Node, cross: i32) -> Option<Line> {
        let (lo, hi) = self.band(self.axis);
        let (n1, n2) = self.axis.span(node.bounds);
        let delta = f64::from(n1 + n2 - lo - hi) / 2.0;

        if delta.abs() < 1.0 || hi <= lo {
            return None;
        }

        let reach = f64::from(hi - lo) * 0.8;
        let travel = (delta.abs() + self.axis.panel(self.size) * SLOP).min(reach) * delta.signum();
        let mid = f64::from(lo + hi) / 2.0;
        let at = |v: f64| match self.axis {
            Axis::X => (v.round() as i32, cross),
            Axis::Y => (cross, v.round() as i32),
        };

        Some((at(mid + travel / 2.0), at(mid - travel / 2.0)))
    }
}

fn sought<'a>(s: &Session, screen: &'a Screen, what: &str) -> Result<Option<&'a Node>> {
    match a11y::pick_in(&screen.nodes, what, None) {
        Ok(node) => Ok(Some(node)),
        Err(e) if e.is::<a11y::Ambiguous>() && screen.nodes.iter().any(|n| n.answers(what)) => {
            s.remember(&screen.nodes);
            Err(e)
        }
        Err(_) => Ok(None),
    }
}

pub async fn until(s: &Session, command: Command) -> Result<()> {
    let Command::Swipe {
        from: Some(from),
        to,
        duration,
        amount,
        force,
        until: Some(what),
        max,
        ..
    } = command
    else {
        bail!("--until repeats a swipe, and this one has no direction or ends");
    };

    if what.starts_with('@') {
        bail!("@index numbers one dump and --until reads many; name the element");
    }

    let amount = match to {
        None if amount == DEFAULT_AMOUNT => SCROLL_AMOUNT,
        _ => amount,
    };
    let over = match duration == SWIPE_DEFAULT {
        true => SCROLL_DURATION,
        false => duration,
    };

    let line = crate::ends(s, &from, to.as_deref(), amount, force).await?;
    let (screen, swipes) = seek(s, &what, line, over, max).await?;
    let (screen, nudges, whole) = settle(s, screen, &what, line).await?;

    report(s, screen, &what, swipes, nudges, whole)
}

fn stopped(before: &[Node], after: &[Node]) -> bool {
    let id = |n: &Node| (n.res_id.clone(), n.class.clone(), n.text.clone(), n.desc.clone());
    let was: HashMap<_, Bounds> = before.iter().map(|n| (id(n), n.bounds)).collect();
    let kept: Vec<bool> = after
        .iter()
        .filter_map(|n| was.get(&id(n)).map(|b| *b == n.bounds))
        .collect();

    kept.len() * 2 > after.len() && kept.iter().all(|&same| same)
}

async fn seek(s: &Session, what: &str, line: Line, over: Duration, max: usize) -> Result<(Screen, usize)> {
    let t = &s.target;
    let mut screen = a11y::dump(t).await?;

    for swipes in 0..max {
        if sought(s, &screen, what)?.is_some() {
            return Ok((screen, swipes));
        }

        a11y::swipe(t, line.0, line.1, over).await?;

        let change = answer::after(t, &screen).await?;
        let stuck = change.how == How::Unmoved || stopped(&screen.nodes, &change.screen.nodes);

        screen = change.screen;

        if stuck {
            bail!(
                "'{what}' is not in this list: it stopped moving after {}{}",
                counted(swipes + 1),
                crate::instead(&screen.nodes, what)
            );
        }
    }

    match sought(s, &screen, what)? {
        Some(_) => Ok((screen, max)),
        None => bail!(
            "'{what}' did not come into view within {max} swipes; --max goes further{}",
            crate::instead(&screen.nodes, what)
        ),
    }
}

async fn settle(s: &Session, mut screen: Screen, what: &str, line: Line) -> Result<(Screen, usize, bool)> {
    let t = &s.target;
    let axis = Axis::of(line);
    let cross = match axis {
        Axis::X => line.0 .1,
        Axis::Y => line.0 .0,
    };
    let size = a11y::size(t).await?;
    let mut nudges = 0;

    loop {
        let Some(node) = sought(s, &screen, what)? else {
            bail!("'{what}' slid out of view while being brought clear{}", crate::instead(&screen.nodes, what));
        };
        let seen = Seen {
            keyboard: screen.keyboard.and_then(|k| k.frame),
            size,
            axis,
        };

        if seen.clear(node) {
            return Ok((screen, nudges, true));
        }

        let Some((a, b)) = seen.nudge(node, cross).filter(|_| nudges < NUDGES) else {
            return Ok((screen, nudges, false));
        };

        a11y::swipe(t, a, b, NUDGE_DURATION).await?;
        screen = answer::after(t, &screen).await?.screen;
        nudges += 1;
    }
}

fn counted(swipes: usize) -> String {
    match swipes {
        1 => "1 swipe".to_string(),
        n => format!("{n} swipes"),
    }
}

fn report(s: &Session, screen: Screen, what: &str, swipes: usize, nudges: usize, whole: bool) -> Result<()> {
    {
        let Some(node) = sought(s, &screen, what)? else {
            bail!("'{what}' left the screen as it was read");
        };
        let rows = a11y::rows(&screen.nodes);
        let at = screen.nodes.iter().position(|n| std::ptr::eq(n, node)).unwrap_or_default();
        let row = &rows[at];
        let row = row.within.map_or(row, |p| &rows[p]);

        let mut shown = a11y::recall(&s.view.device.id).unwrap_or_default();
        let index = answer::number(&mut shown, &[row.node])[0];

        if let Err(e) = a11y::remember_rows(&s.view.device.id, &shown) {
            eprintln!("phone: could not keep these rows for @index: {e:#}");
        }

        let travel = match swipes {
            0 => "already on screen".to_string(),
            n => format!("after {}", counted(n)),
        };
        let nudged = match nudges {
            0 => "",
            _ => ", then a short drag to bring it clear",
        };
        let hidden = match whole {
            true => "",
            false => "; part of it may still be under a bar, the keyboard or the list's edge",
        };

        println!("found      {} {travel}{nudged}{hidden}", node.label());
        crate::print_row(&screen, row, index);
    }

    *s.read.borrow_mut() = Some(screen);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANEL: Size = Size {
        width: 1080.0,
        height: 2400.0,
        scale: 1.0,
    };

    fn b(x1: i32, y1: i32, x2: i32, y2: i32) -> Bounds {
        Bounds { x1, y1, x2, y2 }
    }

    fn node(bounds: Bounds, ancestors: Vec<Bounds>) -> Node {
        Node {
            index: 0,
            text: "Row".to_string(),
            desc: String::new(),
            res_id: String::new(),
            class: "android.widget.TextView".to_string(),
            clickable: false,
            bounds,
            ancestors,
            focused: false,
            hint: String::new(),
            password: false,
            parent: None,
        }
    }

    fn seen(keyboard: Option<Bounds>, axis: Axis) -> Seen {
        Seen {
            keyboard,
            size: PANEL,
            axis,
        }
    }

    fn list(rows: &[(&str, i32)]) -> Vec<Node> {
        rows.iter()
            .map(|&(text, y)| Node {
                text: text.to_string(),
                ..node(b(0, y, 1080, y + 130), vec![])
            })
            .collect()
    }

    #[test]
    fn a_list_has_stopped_when_what_it_kept_stayed_put_whatever_else_ticks() {
        let before = list(&[("log 1", 0), ("Row 8", 200), ("Row 9", 330), ("Row 10", 460)]);

        for (after, want) in [
            (list(&[("log 2", 0), ("Row 8", 200), ("Row 9", 330), ("Row 10", 460)]), true),
            (list(&[("log 1", 0), ("Row 9", 200), ("Row 10", 330), ("Row 11", 460)]), false),
            (list(&[("log 1", 0), ("Row 40", 200), ("Row 41", 330), ("Row 42", 460)]), false),
        ] {
            assert_eq!(stopped(&before, &after), want);
        }
    }

    #[test]
    fn edges_start_on_the_outermost_pixel_and_drag_inward() {
        for (edge, want) in [
            (Edge::Left, ((0, 1200), (647, 1200))),
            (Edge::Right, ((1079, 1200), (432, 1200))),
            (Edge::Top, ((540, 0), (540, 1439))),
            (Edge::Bottom, ((540, 2399), (540, 960))),
        ] {
            assert_eq!(inward(PANEL, edge, 0.6), want, "{edge:?}");
        }
    }

    #[test]
    fn a_row_is_clear_only_inside_its_list_the_bars_and_above_the_keyboard() {
        let list = b(0, 300, 1080, 2200);
        let row = |y1: i32, y2: i32| node(b(40, y1, 400, y2), vec![b(0, (y1 - 20).max(300), 1080, (y2 + 20).min(2200)), list]);
        let keyboard = Some(b(0, 1500, 1080, 2400));

        for (n, keyboard, axis, want) in [
            (row(1000, 1100), None, Axis::Y, true),
            (row(300, 380), None, Axis::Y, false),
            (row(2120, 2200), None, Axis::Y, false),
            (row(1600, 1700), keyboard, Axis::Y, false),
            (row(1600, 1700), None, Axis::Y, true),
            (node(b(0, 10, 1080, 90), vec![]), None, Axis::Y, false),
            (node(b(900, 1000, 1080, 1300), vec![b(0, 1000, 1080, 1300)]), None, Axis::X, false),
            (node(b(700, 1000, 1000, 1300), vec![b(0, 1000, 1080, 1300)]), None, Axis::X, true),
        ] {
            assert_eq!(seen(keyboard, axis).clear(&n), want, "{:?} {axis:?}", n.bounds);
        }
    }

    #[test]
    fn a_nudge_drags_the_content_toward_the_middle_of_its_room() {
        let list = b(0, 300, 1080, 2200);
        let carousel = b(0, 1000, 1080, 1300);

        for (n, axis, cross, want) in [
            (node(b(40, 2150, 400, 2200), vec![list]), Axis::Y, 540, Some(((540, 1700), (540, 701)))),
            (node(b(40, 301, 400, 330), vec![list]), Axis::Y, 540, Some(((540, 746), (540, 1654)))),
            (node(b(40, 1160, 400, 1240), vec![list]), Axis::Y, 540, None),
            (node(b(1000, 1000, 1079, 1300), vec![carousel]), Axis::X, 1150, Some(((795, 1150), (285, 1150)))),
        ] {
            assert_eq!(seen(None, axis).nudge(&n, cross), want, "{:?}", n.bounds);
        }
    }

    #[test]
    fn the_axis_is_the_longer_leg_of_the_line() {
        for (line, want) in [
            (((540, 1800), (540, 700)), Axis::Y),
            (((900, 640), (200, 640)), Axis::X),
            (((900, 640), (800, 1640)), Axis::Y),
        ] {
            assert_eq!(Axis::of(line), want);
        }
    }
}
