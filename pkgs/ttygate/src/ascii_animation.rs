use std::sync::OnceLock;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};

use crate::bg::carousel::Carousel;
use crate::bg::noise::bayer;
use crate::bg::{self, Cell};
use crate::theme::Theme;

static CAROUSEL: OnceLock<Carousel> = OnceLock::new();

pub fn frame(tick: u64, w: u16, h: u16, theme: &Theme) -> Text<'static> {
    frame_with(CAROUSEL.get_or_init(Carousel::from_clock), tick, w, h, theme)
}

pub fn current_name(tick: u64) -> &'static str {
    CAROUSEL.get_or_init(Carousel::from_clock).at(tick).current.name
}

pub fn frame_with(
    carousel: &Carousel,
    tick: u64,
    w: u16,
    h: u16,
    theme: &Theme,
) -> Text<'static> {
    let (w, h) = (w as usize, h as usize);
    if w == 0 || h == 0 {
        return Text::default();
    }

    let slot = carousel.at(tick);
    let mut cells = bg::render(slot.current.effect, tick as f32 * slot.current.speed, w, h);

    if let Some((prev, progress)) = slot.incoming_from {
        let old = bg::render(prev.effect, tick as f32 * prev.speed, w, h);
        dissolve(&mut cells, &old, progress, w);
    }

    let mut lines = Vec::with_capacity(h);
    for row in cells.chunks_exact(w) {
        lines.push(row_to_line(row, theme));
    }
    Text::from(lines)
}

fn dissolve(into: &mut [Cell], from: &[Cell], progress: f32, w: usize) {
    for (i, cell) in into.iter_mut().enumerate() {
        if bayer(i % w, i / w) >= progress {
            *cell = from[i];
        }
    }
}

fn row_to_line(row: &[Cell], theme: &Theme) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    let mut run = String::new();
    let mut run_style = Style::default();
    let mut started = false;

    for cell in row {
        let style = Style::default().fg(lerp(theme.dim, theme.accent, cell.v));
        if started && style == run_style {
            run.push(cell.ch);
        } else {
            if started {
                spans.push(Span::styled(std::mem::take(&mut run), run_style));
            }
            run.push(cell.ch);
            run_style = style;
            started = true;
        }
    }
    if started {
        spans.push(Span::styled(run, run_style));
    }
    Line::from(spans)
}

fn lerp(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            Color::Rgb(mix(ar, br, t), mix(ag, bg, t), mix(ab, bb, t))
        }
        _ => b,
    }
}

fn mix(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bg::carousel::SLOT;
    use crate::bg::effects::ENTRIES;
    use crate::theme::{Accent, Theme};

    fn plain(text: &Text) -> String {
        text.lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect()
    }

    #[test]
    fn animates_between_ticks() {
        let th = Theme::preset(Accent::Amber);
        assert_ne!(
            plain(&frame(10, 80, 24, &th)),
            plain(&frame(11, 80, 24, &th)),
            "consecutive ticks must move the background"
        );
    }

    #[test]
    fn every_effect_animates_after_the_fold() {
        let th = Theme::preset(Accent::Amber);
        let c = Carousel::new(0);
        for i in 0..ENTRIES.len() as u64 {
            let t = i * SLOT + SLOT / 2;
            let name = c.at(t).current.name;
            assert_ne!(
                plain(&frame_with(&c, t, 80, 24, &th)),
                plain(&frame_with(&c, t + 1, 80, 24, &th)),
                "{name} does not move between consecutive greeter frames"
            );
        }
    }

    #[test]
    fn fills_exact_dimensions() {
        let th = Theme::preset(Accent::Amber);
        let f = frame(0, 64, 20, &th);
        assert_eq!(f.lines.len(), 20);
        for l in &f.lines {
            assert_eq!(l.width(), 64, "every row must span the full width");
        }
    }

    #[test]
    fn fills_exact_dimensions_mid_dissolve() {
        let th = Theme::preset(Accent::Amber);
        let c = Carousel::new(3);
        for t in SLOT..SLOT + 3 {
            let f = frame_with(&c, t, 64, 20, &th);
            assert_eq!(f.lines.len(), 20);
            for l in &f.lines {
                assert_eq!(l.width(), 64, "row not full width at tick {t}");
            }
        }
    }

    #[test]
    fn zero_area_is_empty() {
        let th = Theme::preset(Accent::Amber);
        assert_eq!(frame(0, 0, 0, &th).lines.len(), 0);
    }

    #[test]
    fn a_slot_boundary_changes_the_picture() {
        let th = Theme::preset(Accent::Amber);
        let c = Carousel::new(2024);
        let before = plain(&frame_with(&c, SLOT - 1, 80, 24, &th));
        let after = plain(&frame_with(&c, SLOT + 20, 80, 24, &th));
        assert_ne!(before, after, "the background did not change across a slot");
    }
}
