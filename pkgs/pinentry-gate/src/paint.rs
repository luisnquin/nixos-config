use ttycanvas::carousel::Carousel;
use ttycanvas::noise::bayer;
use ttycanvas::{self as bg, Cell};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub const ACCENT: Rgb = Rgb(255, 176, 0);
pub const FG: Rgb = Rgb(230, 224, 214);
pub const DIM: Rgb = Rgb(120, 110, 92);
pub const ERROR: Rgb = Rgb(255, 90, 60);
pub const BG: Rgb = Rgb(10, 10, 12);

#[derive(Clone, Copy, PartialEq)]
pub struct Ink {
    pub ch: char,
    pub fg: Rgb,
    pub bold: bool,
}

impl Default for Ink {
    fn default() -> Self {
        Ink { ch: ' ', fg: BG, bold: false }
    }
}

impl Ink {
    pub fn new(ch: char, fg: Rgb) -> Self {
        Ink { ch, fg, bold: false }
    }

    pub fn bold(ch: char, fg: Rgb) -> Self {
        Ink { ch, fg, bold: true }
    }
}

fn mix(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    Rgb(mix(a.0, b.0, t), mix(a.1, b.1, t), mix(a.2, b.2, t))
}

pub fn background(carousel: &Carousel, tick: u64, w: usize, h: usize) -> Vec<Ink> {
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let slot = carousel.at(tick);
    let mut cells = bg::render(slot.current.effect, tick as f32 * slot.current.speed, w, h);
    if let Some((prev, progress)) = slot.incoming_from {
        let old = bg::render(prev.effect, tick as f32 * prev.speed, w, h);
        for (i, cell) in cells.iter_mut().enumerate() {
            if bayer(i % w, i / w) >= progress {
                *cell = old[i];
            }
        }
    }
    cells.iter().map(shade).collect()
}

fn shade(cell: &Cell) -> Ink {
    Ink::new(cell.ch, lerp(DIM, ACCENT, cell.v))
}

pub struct Screen {
    pub w: usize,
    pub h: usize,
    shown: Option<Vec<Ink>>,
}

impl Screen {
    pub fn new(w: usize, h: usize) -> Self {
        Screen { w, h, shown: None }
    }

    #[cfg(test)]
    pub fn blank(&self) -> Vec<Ink> {
        vec![Ink::default(); self.w * self.h]
    }

    pub fn flush(&mut self, next: &[Ink]) -> Vec<u8> {
        let mut out = Vec::new();
        if self.w == 0 || self.h == 0 || next.len() != self.w * self.h {
            return out;
        }
        if self.shown.is_none() {
            out.extend_from_slice(b"\x1b[?25l\x1b[2J");
        }
        for y in 0..self.h {
            let row = &next[y * self.w..(y + 1) * self.w];
            for (lo, hi) in runs(row, self.shown.as_ref().map(|s| &s[y * self.w..(y + 1) * self.w])) {
                out.extend_from_slice(format!("\x1b[{};{}H", y + 1, lo + 1).as_bytes());
                emit(&mut out, &row[lo..=hi]);
            }
        }
        out.extend_from_slice(b"\x1b[0m");
        self.shown = Some(next.to_vec());
        out
    }
}

const JUMP: usize = 8;

fn runs(row: &[Ink], shown: Option<&[Ink]>) -> Vec<(usize, usize)> {
    let Some(shown) = shown else {
        return vec![(0, row.len() - 1)];
    };
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, _) in row.iter().zip(shown).enumerate().filter(|(_, (a, b))| a != b) {
        match out.last_mut() {
            Some((_, hi)) if i - *hi <= JUMP => *hi = i,
            _ => out.push((i, i)),
        }
    }
    out
}

fn emit(out: &mut Vec<u8>, run: &[Ink]) {
    let mut style: Option<(Rgb, bool)> = None;
    let mut text = String::new();
    for ink in run {
        let want = (ink.fg, ink.bold);
        if style != Some(want) {
            out.extend_from_slice(text.as_bytes());
            text.clear();
            let Rgb(r, g, b) = ink.fg;
            let weight = if ink.bold { "1" } else { "22" };
            out.extend_from_slice(format!("\x1b[{weight};38;2;{r};{g};{b}m").as_bytes());
            style = Some(want);
        }
        text.push(ink.ch);
    }
    out.extend_from_slice(text.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(chars: &str, fg: Rgb) -> Vec<Ink> {
        chars.chars().map(|c| Ink::new(c, fg)).collect()
    }

    #[test]
    fn the_first_flush_clears_and_paints_everything() {
        let mut screen = Screen::new(4, 2);
        let out = String::from_utf8(screen.flush(&screen.blank())).unwrap();
        assert!(out.contains("\x1b[2J"), "no clear on the first frame");
        assert!(out.contains("\x1b[?25l"), "cursor left visible");
        assert!(out.contains("\x1b[1;1H") && out.contains("\x1b[2;1H"));
    }

    #[test]
    fn an_unchanged_frame_costs_nothing() {
        let mut screen = Screen::new(8, 3);
        let frame = screen.blank();
        screen.flush(&frame);
        let out = screen.flush(&frame);
        assert_eq!(out, b"\x1b[0m", "a still frame repainted");
    }

    #[test]
    fn only_the_changed_span_is_repainted() {
        let mut screen = Screen::new(8, 1);
        let before = row("........", DIM);
        screen.flush(&before);
        let mut after = before.clone();
        after[3] = Ink::new('#', ACCENT);
        after[5] = Ink::new('#', ACCENT);
        let out = String::from_utf8(screen.flush(&after)).unwrap();
        assert!(out.contains("\x1b[1;4H"), "did not seek to the first change");
        assert!(out.contains('#'));
        assert_eq!(out.matches('.').count(), 1, "repainted beyond the dirty span");
    }

    #[test]
    fn one_escape_per_colour_run() {
        let mut screen = Screen::new(6, 1);
        screen.flush(&screen.blank());
        let mut next = row("aaabbb", ACCENT);
        for ink in next.iter_mut().take(3) {
            ink.fg = DIM;
        }
        let out = String::from_utf8(screen.flush(&next)).unwrap();
        assert_eq!(out.matches("38;2;").count(), 2, "a run was not coalesced");
    }

    #[test]
    fn a_mismatched_frame_is_refused() {
        let mut screen = Screen::new(4, 2);
        assert!(screen.flush(&[Ink::default(); 3]).is_empty());
    }

    #[test]
    fn the_background_fills_every_cell_and_moves() {
        let carousel = Carousel::new(11);
        let a = background(&carousel, 10, 40, 12);
        let b = background(&carousel, 11, 40, 12);
        assert_eq!(a.len(), 40 * 12);
        assert_ne!(
            a.iter().map(|i| i.ch).collect::<String>(),
            b.iter().map(|i| i.ch).collect::<String>(),
            "the background did not move between ticks"
        );
    }

    #[test]
    fn a_zero_area_background_is_empty() {
        assert!(background(&Carousel::new(0), 0, 0, 0).is_empty());
    }
}
