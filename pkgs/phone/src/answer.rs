use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use anyhow::Result;
use image::RgbImage;

use crate::a11y::{self, Bounds, Keyboard, Node, Row, Screen, Signature, Target};
use crate::actions::{MOVE_LIMIT, SETTLE_LIMIT, STILL_SHARE};

const STEP: Duration = Duration::from_millis(350);

#[derive(Debug, PartialEq)]
pub enum How {
    Moved,
    Unmoved,
    Restless,
}

pub struct Change {
    pub screen: Screen,
    pub how: How,
    pub appeared: Vec<usize>,
    pub gone: Vec<String>,
    pub changing: Vec<String>,
}

#[derive(Default, PartialEq)]
struct Look {
    rows: Vec<Signature>,
    keyboard: Option<Keyboard>,
}

fn look(screen: &Screen) -> Look {
    Look {
        rows: screen.nodes.iter().map(Node::signature).collect(),
        keyboard: screen.keyboard,
    }
}

pub async fn after(t: &Target, before: &Screen) -> Result<Change> {
    let started = Instant::now();
    let first = a11y::dump(t).await?;
    let mut settle = Settle::new(look(before), look(&first));
    let mut took = started.elapsed();

    loop {
        tokio::time::sleep(STEP.saturating_sub(took)).await;

        let read = Instant::now();
        let screen = a11y::dump(t).await?;
        took = read.elapsed();

        let how = match settle.see(look(&screen), started.elapsed()) {
            Verdict::Still => continue,
            Verdict::Done => How::Moved,
            Verdict::Unmoved => How::Unmoved,
            Verdict::Restless => How::Restless,
        };

        let changing = settle.changing(&how);

        return Ok(Change {
            changing: changing.iter().map(named).collect(),
            ..compare(before, screen, how, &changing)
        });
    }
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Still,
    Done,
    Unmoved,
    Restless,
}

/// A row already changing before the act, a video or a live preview, is read
/// past; one that moves must be seen at it twice, since a slide moves them all.
struct Settle {
    before: Look,
    previous: Look,
    first: Vec<Signature>,
    since: Vec<Signature>,
    background: Vec<Signature>,
    last: Vec<Signature>,
    moved: bool,
}

impl Settle {
    fn new(before: Look, first: Look) -> Self {
        Settle {
            moved: before != first,
            first: differ(&before.rows, &first.rows),
            before,
            previous: first,
            since: Vec::new(),
            background: Vec::new(),
            last: Vec::new(),
        }
    }

    fn see(&mut self, next: Look, elapsed: Duration) -> Verdict {
        self.moved |= self.before != next;

        let diff = differ(&self.previous.rows, &next.rows);
        let (background, rest): (Vec<Signature>, Vec<Signature>) =
            diff.iter().cloned().partition(|r| self.ambient(r));
        let quiet = rest.is_empty() && next.keyboard == self.previous.keyboard;

        if !background.is_empty() {
            self.background.clear();
            background
                .into_iter()
                .for_each(|r| keep(&mut self.background, r));
        }

        self.last = rest;
        self.since.extend(diff);
        self.previous = next;

        match (quiet, self.moved) {
            (true, true) => Verdict::Done,
            (true, false) if elapsed >= MOVE_LIMIT => Verdict::Unmoved,
            _ if elapsed >= SETTLE_LIMIT => Verdict::Restless,
            _ => Verdict::Still,
        }
    }

    fn ambient(&self, row: &Signature) -> bool {
        self.before.rows.iter().any(|b| akin(b, row))
            && (self.first.iter().any(|f| in_place(f, row))
                || self.since.iter().any(|s| akin(s, row)))
    }

    fn changing(&self, how: &How) -> Vec<Signature> {
        let mut rows: Vec<Signature> = self
            .background
            .iter()
            .filter(|r| !self.before.rows.contains(r))
            .cloned()
            .collect();

        if *how == How::Restless {
            for row in self.last.iter().cloned() {
                keep(&mut rows, row);
            }
        }

        rows
    }
}

fn in_place(a: &Signature, b: &Signature) -> bool {
    a.class == b.class && a.res_id == b.res_id && a.bounds == b.bounds
}

fn akin(a: &Signature, b: &Signature) -> bool {
    let named = !(a.text.is_empty() && a.desc.is_empty());
    let same = a.class == b.class && a.res_id == b.res_id && a.text == b.text && a.desc == b.desc;

    in_place(a, b) || (named && same)
}

fn differ(was: &[Signature], now: &[Signature]) -> Vec<Signature> {
    let gone = unmatched(was, now, Signature::clone);
    let came = unmatched(now, was, Signature::clone);

    gone.into_iter().chain(came).cloned().collect()
}

fn keep(rows: &mut Vec<Signature>, row: Signature) {
    match rows.iter_mut().find(|r| akin(r, &row)) {
        Some(r) => *r = row,
        None => rows.push(row),
    }
}

fn named(s: &Signature) -> String {
    let b = s.bounds;
    let name = [&s.text, &s.desc, &s.res_id]
        .into_iter()
        .find(|f| !f.is_empty())
        .map(|f| a11y::row_label(f))
        .unwrap_or_else(|| format!("<{}>", s.class.rsplit('.').next().unwrap_or(&s.class)));

    format!("{name} [{},{}][{},{}]", b.x1, b.y1, b.x2, b.y2)
}

pub fn compare(before: &Screen, screen: Screen, how: How, noise: &[Signature]) -> Change {
    let was = listed(before, noise);
    let now = listed(&screen, noise);
    let appeared = fresh(&now, &was).iter().map(|r| r.node.index).collect();
    let gone = fresh(&was, &now).iter().map(|r| r.label.clone()).collect();

    Change {
        screen,
        how,
        appeared,
        gone,
        changing: Vec::new(),
    }
}

type Identity<'a> = (&'a str, &'a str, &'a str, &'a str);

fn identity<'a>(r: &Row<'a>) -> Identity<'a> {
    (&r.node.res_id, &r.node.class, &r.node.text, &r.node.desc)
}

fn listed<'a>(screen: &'a Screen, noise: &[Signature]) -> Vec<Row<'a>> {
    a11y::rows(&screen.nodes)
        .into_iter()
        .filter(|r| r.within.is_none())
        .filter(|r| {
            let row = r.node.signature();
            !noise.iter().any(|n| akin(n, &row))
        })
        .collect()
}

/// Rows of `from` that `against` has no counterpart for. Where a row sits is
/// left out, so a scroll reports what came into view rather than every row.
fn fresh<'a, 'b>(from: &'b [Row<'a>], against: &[Row<'a>]) -> Vec<&'b Row<'a>> {
    unmatched(from, against, identity)
}

fn unmatched<'b, T, K: Eq + Hash>(
    from: &'b [T],
    against: &[T],
    key: impl Fn(&T) -> K,
) -> Vec<&'b T> {
    let mut left: HashMap<K, usize> = HashMap::new();

    for n in against {
        *left.entry(key(n)).or_default() += 1;
    }

    from.iter()
        .filter(|n| match left.get_mut(&key(n)) {
            Some(count) if *count > 0 => {
                *count -= 1;
                false
            }
            _ => true,
        })
        .collect()
}

pub fn number(shown: &mut Vec<Signature>, rows: &[&Node]) -> Vec<usize> {
    rows.iter()
        .map(|n| {
            let row = n.signature();

            shown.iter().position(|s| *s == row).unwrap_or_else(|| {
                shown.push(row);
                shown.len() - 1
            })
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum Drawn {
    Still,
    Reshaped,
    At(Bounds),
}

/// Where two frames differ, in pixels. Too few changed pixels to be anything a
/// tap caused (a blinking cursor, a spinner) reads as still.
pub fn drawn(before: &RgbImage, after: &RgbImage) -> Drawn {
    if before.dimensions() != after.dimensions() {
        return Drawn::Reshaped;
    }

    let mut changed = 0u64;
    let mut at = Bounds {
        x1: i32::MAX,
        y1: i32::MAX,
        x2: 0,
        y2: 0,
    };

    for ((x, y, p), q) in before.enumerate_pixels().zip(after.pixels()) {
        if p.0.iter().zip(q.0).any(|(a, b)| a.abs_diff(b) > 16) {
            changed += 1;
            at.x1 = at.x1.min(x as i32);
            at.y1 = at.y1.min(y as i32);
            at.x2 = at.x2.max(x as i32 + 1);
            at.y2 = at.y2.max(y as i32 + 1);
        }
    }

    let (w, h) = before.dimensions();

    match (changed as f64) < STILL_SHARE * f64::from(w) * f64::from(h) {
        true => Drawn::Still,
        false => Drawn::At(at),
    }
}

pub fn in_points(px: Bounds, scale: f64) -> Bounds {
    let down = |v: i32| (f64::from(v) / scale).floor() as i32;
    let up = |v: i32| (f64::from(v) / scale).ceil() as i32;

    Bounds {
        x1: down(px.x1),
        y1: down(px.y1),
        x2: up(px.x2),
        y2: up(px.y2),
    }
}

pub fn holder<'r, 'a>(rows: &'r [Row<'a>], at: Bounds) -> Option<&'r Row<'a>> {
    rows.iter()
        .rev()
        .filter(|r| {
            let b = r.node.bounds;

            b.x1 <= at.x1 && b.y1 <= at.y1 && b.x2 >= at.x2 && b.y2 >= at.y2
        })
        .min_by_key(|r| i64::from(r.node.bounds.width()) * i64::from(r.node.bounds.height()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(xml: &str) -> Screen {
        Screen {
            nodes: a11y::parse(xml).unwrap(),
            keyboard: None,
            panel: None,
        }
    }

    fn row(text: &str, clickable: bool, y: i32) -> String {
        format!(
            r#"<node class="android.widget.TextView" bounds="[40,{y}][1040,{}]" clickable="{clickable}" text="{text}" content-desc="" resource-id=""/>"#,
            y + 80
        )
    }

    fn hierarchy(rows: &[String]) -> String {
        format!(
            r#"<hierarchy rotation="0"><node class="android.widget.FrameLayout" bounds="[0,0][1080,2400]" clickable="false" text="" content-desc="" resource-id="">{}</node></hierarchy>"#,
            rows.concat()
        )
    }

    const TABS: [&str; 4] = ["Inicio", "Operaciones", "Préstamos", "Configuración"];

    fn home() -> Screen {
        let mut rows = vec![
            row("Operaciones en curso", false, 360),
            row("Viaje QA Cusco, colecta fija", true, 740),
            row("Tus préstamos", false, 1180),
            row("Ver Con meta fija", true, 2090),
        ];
        rows.extend(TABS.iter().map(|t| row(t, true, 2200)));

        screen(&hierarchy(&rows))
    }

    fn form() -> Screen {
        let mut rows = vec![
            row("Nueva colecta", false, 200),
            row("Nombre", true, 500),
            row("Meta", true, 700),
            row("Continuar", true, 2090),
        ];
        rows.extend(TABS.iter().map(|t| row(t, true, 2200)));

        screen(&hierarchy(&rows))
    }

    #[test]
    fn a_tap_that_opens_a_screen_names_what_came_and_what_went() {
        let change = compare(&home(), form(), How::Moved, &[]);
        let labels = |at: &[usize]| -> Vec<String> {
            at.iter().map(|&i| change.screen.nodes[i].label()).collect()
        };

        assert_eq!(
            labels(&change.appeared),
            ["Nueva colecta", "Nombre", "Meta", "Continuar"]
        );
        assert_eq!(
            change.gone,
            [
                "Operaciones en curso",
                "Viaje QA Cusco, colecta fija",
                "Tus préstamos",
                "Ver Con meta fija"
            ]
        );
    }

    #[test]
    fn a_scroll_reports_the_rows_that_came_into_view_not_the_ones_that_moved() {
        let before = screen(&hierarchy(&[row("A", false, 100), row("B", false, 300)]));
        let after = screen(&hierarchy(&[row("B", false, 100), row("C", false, 300)]));

        let change = compare(&before, after, How::Moved, &[]);

        assert_eq!(change.appeared, [1]);
        assert_eq!(change.gone, ["A"]);
    }

    #[test]
    fn a_second_row_of_one_name_counts_as_new() {
        let before = screen(&hierarchy(&[row("Item", false, 100)]));
        let after = screen(&hierarchy(&[
            row("Item", false, 100),
            row("Item", false, 300),
        ]));

        assert_eq!(compare(&before, after, How::Moved, &[]).appeared, [1]);
    }

    fn at(text: &str, y: i32) -> Signature {
        Signature {
            res_id: String::new(),
            class: "android.widget.TextView".to_string(),
            text: text.to_string(),
            desc: String::new(),
            bounds: a11y::Bounds {
                x1: 40,
                y1: y,
                x2: 1040,
                y2: y + 80,
            },
        }
    }

    fn seen(rows: &[Signature]) -> Look {
        Look {
            rows: rows.to_vec(),
            keyboard: None,
        }
    }

    const SOON: Duration = Duration::from_millis(900);

    #[test]
    fn a_screen_that_holds_still_after_the_act_is_done_on_the_next_read() {
        let mut settle = Settle::new(seen(&[at("Home", 100)]), seen(&[at("Form", 100)]));

        assert_eq!(settle.see(seen(&[at("Form", 100)]), SOON), Verdict::Done);
        assert!(settle.changing(&How::Moved).is_empty());
    }

    #[test]
    fn a_video_playing_before_the_act_is_read_past_and_named() {
        let mut settle = Settle::new(
            seen(&[at("Title", 100), at("0:01", 900)]),
            seen(&[at("Title", 100), at("Next", 300), at("0:02", 900)]),
        );

        let verdict = settle.see(
            seen(&[at("Title", 100), at("Next", 300), at("0:03", 900)]),
            SOON,
        );

        assert_eq!(verdict, Verdict::Done);
        assert_eq!(
            settle
                .changing(&How::Moved)
                .iter()
                .map(named)
                .collect::<Vec<_>>(),
            ["0:03 [40,900][1040,980]"]
        );
    }

    #[test]
    fn a_row_back_as_it_was_after_an_empty_read_is_not_named() {
        let mut settle = Settle::new(seen(&[at("Title", 100), at("0:01", 900)]), seen(&[]));

        let verdict = settle.see(seen(&[at("Title", 100), at("0:02", 900)]), SOON);

        assert_eq!(verdict, Verdict::Done);
        assert_eq!(
            settle
                .changing(&How::Moved)
                .iter()
                .map(named)
                .collect::<Vec<_>>(),
            ["0:02 [40,900][1040,980]"]
        );
    }

    #[test]
    fn background_rows_are_left_out_of_what_came_and_went() {
        let before = screen(&hierarchy(&[
            row("Title", false, 100),
            row("0:01", false, 900),
        ]));
        let after = screen(&hierarchy(&[
            row("Title", false, 100),
            row("Next", true, 300),
            row("0:03", false, 900),
        ]));
        let noise = [after.nodes[2].signature()];

        let change = compare(&before, after, How::Moved, &noise);

        assert_eq!(change.appeared, [1]);
        assert!(change.gone.is_empty());
    }

    #[test]
    fn a_row_the_act_brought_in_is_waited_for_and_named_at_the_limit() {
        let mut settle = Settle::new(
            seen(&[at("Title", 100)]),
            seen(&[at("Title", 100), at("Loading 10%", 300)]),
        );

        let busy = |n: &str| seen(&[at("Title", 100), at(n, 300)]);

        assert_eq!(settle.see(busy("Loading 40%"), SOON), Verdict::Still);
        assert_eq!(
            settle.see(busy("Loading 70%"), SETTLE_LIMIT),
            Verdict::Restless
        );
        assert_eq!(
            settle
                .changing(&How::Restless)
                .iter()
                .map(named)
                .collect::<Vec<_>>(),
            ["Loading 70% [40,300][1040,380]"]
        );
    }

    fn verdicts(before: &[Signature], first: &[Signature], reads: &[&[Signature]]) -> Vec<Verdict> {
        let mut settle = Settle::new(seen(before), seen(first));

        reads.iter().map(|r| settle.see(seen(r), SOON)).collect()
    }

    #[test]
    fn a_slide_on_the_first_read_is_not_taken_for_background() {
        let settled = [at("A", 0), at("B", 200)];

        assert_eq!(
            verdicts(
                &[at("A", 100), at("B", 300)],
                &[at("A", 50), at("B", 250)],
                &[&settled, &settled]
            ),
            [Verdict::Still, Verdict::Done]
        );
    }

    #[test]
    fn a_row_seen_moving_twice_after_the_act_is_background() {
        assert_eq!(
            verdicts(
                &[at("Ball", 100), at("Title", 0)],
                &[at("Ball", 200), at("Title", 0)],
                &[
                    &[at("Ball", 300), at("Title", 0)],
                    &[at("Ball", 400), at("Title", 0)]
                ],
            ),
            [Verdict::Still, Verdict::Done]
        );
    }

    #[test]
    fn a_nameless_row_is_only_matched_where_it_sits() {
        let mut settle = Settle::new(seen(&[at("", 100)]), seen(&[at("", 200)]));

        assert_eq!(settle.see(seen(&[at("", 300)]), SOON), Verdict::Still);
        assert_eq!(settle.see(seen(&[at("", 400)]), SOON), Verdict::Still);
    }

    #[test]
    fn an_act_that_changed_nothing_is_unmoved_once_the_move_limit_passes() {
        let still = || seen(&[at("Home", 100)]);
        let mut settle = Settle::new(still(), still());

        assert_eq!(settle.see(still(), SOON), Verdict::Still);
        assert_eq!(settle.see(still(), MOVE_LIMIT), Verdict::Unmoved);
    }

    #[test]
    fn a_keyboard_coming_up_keeps_the_read_going() {
        let rows = [at("Name", 100)];
        let up = Look {
            rows: rows.to_vec(),
            keyboard: Some(Keyboard {
                shown: true,
                frame: None,
            }),
        };
        let mut settle = Settle::new(seen(&rows), seen(&rows));

        assert_eq!(settle.see(up, SOON), Verdict::Still);
    }

    #[test]
    fn new_rows_are_numbered_after_the_last_snapshot_and_old_ones_keep_theirs() {
        let home = home();
        let form = form();
        let mut shown: Vec<Signature> = home.nodes.iter().map(Node::signature).collect();

        let back: Vec<&Node> = vec![&home.nodes[3], &form.nodes[0]];
        let at = number(&mut shown, &back);

        assert_eq!(at, [3, home.nodes.len()]);
        assert_eq!(shown.len(), home.nodes.len() + 1);
        assert_eq!(
            a11y::pick_in(&form.nodes, &format!("@{}", at[1]), Some(&shown))
                .unwrap()
                .label(),
            "Nueva colecta"
        );
    }

    fn frame(w: u32, h: u32, blots: &[(u32, u32, u32, u32)]) -> RgbImage {
        let mut image = RgbImage::from_pixel(w, h, image::Rgb([240, 240, 240]));

        for &(x, y, bw, bh) in blots {
            for (px, py) in (x..x + bw).flat_map(|px| (y..y + bh).map(move |py| (px, py))) {
                image.put_pixel(px, py, image::Rgb([20, 20, 20]));
            }
        }

        image
    }

    #[test]
    fn frames_are_diffed_into_the_box_that_changed() {
        let blank = frame(100, 100, &[]);
        let at = |x1, y1, x2, y2| Drawn::At(Bounds { x1, y1, x2, y2 });

        for (after, want) in [
            (frame(100, 100, &[]), Drawn::Still),
            (frame(100, 100, &[(50, 50, 2, 2)]), Drawn::Still),
            (frame(100, 100, &[(10, 20, 15, 12)]), at(10, 20, 25, 32)),
            (frame(100, 100, &[(5, 5, 6, 6), (70, 80, 6, 6)]), at(5, 5, 76, 86)),
            (frame(200, 50, &[]), Drawn::Reshaped),
        ] {
            assert_eq!(drawn(&blank, &after), want);
        }
    }

    #[test]
    fn pixels_map_back_to_points_rounding_outward() {
        let px = Bounds {
            x1: 31,
            y1: 30,
            x2: 62,
            y2: 61,
        };

        assert_eq!(
            in_points(px, 3.0),
            Bounds {
                x1: 10,
                y1: 10,
                x2: 21,
                y2: 21
            }
        );
        assert_eq!(in_points(px, 1.0), px);
    }

    #[test]
    fn the_holder_is_the_smallest_row_around_the_change() {
        let screen = screen(&hierarchy(&[
            r#"<node class="android.widget.FrameLayout" bounds="[0,200][1080,1400]" clickable="false" text="" content-desc="Remote screen" resource-id="viewer"><node class="android.widget.ImageView" bounds="[0,300][1080,1300]" clickable="true" text="" content-desc="" resource-id=""/></node>"#.to_string(),
            row("Home", true, 1500),
        ]));
        let rows = a11y::rows(&screen.nodes);

        let inside = |b| holder(&rows, b).map(|r| r.node.kind().to_string());

        let change = Bounds {
            x1: 100,
            y1: 400,
            x2: 300,
            y2: 500,
        };

        assert_eq!(inside(change).as_deref(), Some("ImageView"));
        assert_eq!(
            inside(Bounds { y2: 1350, ..change }).as_deref(),
            Some("FrameLayout")
        );
        assert_eq!(inside(Bounds { x2: 2000, ..change }), None);
    }
}
