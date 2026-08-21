//! The permanent procedural ASCII animation drawn behind the login box. The
//! current effect is a demoscene-style "fly through a round tunnel": every cell
//! is a pure function of its position and `tick`, so there is no state and
//! nothing to hand-author — the motion is continuous by construction.
//!
//! For each cell we take polar coordinates from the screen center, turn radius
//! into a perspective `depth` (`k / r`), and read a shading value off a sine of
//! `depth + time` (rings rushing inward) mixed with the angle (a slow spiral
//! twist). The value picks both an ASCII shade glyph and a color between the
//! theme's dim and accent, so density and hue reinforce the same depth cue.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};

use crate::theme::Theme;

/// Dark→bright shading ramp. The leading space lets the faintest bands fall away
/// into the background instead of drawing a floor of dots.
const RAMP: &[u8] = b" .:-=+*#%@";

/// Terminal cells are roughly twice as tall as wide; scaling vertical distance
/// keeps the rings looking circular rather than as tall ellipses.
const ASPECT: f32 = 2.0;
/// Tunnel "length": larger pushes the ring texture further out toward the mouth.
const DEPTH_SCALE: f32 = 22.0;
/// Ring spacing along the depth axis (how many bright bands you fly through).
const RING_FREQ: f32 = 1.35;
/// Number of angular sectors — the spokes that cross the rings into a grid, so
/// even the large near-edge cells alternate instead of washing flat.
const SPOKES: f32 = 7.0;
/// Couples depth into the angle so the grid spirals instead of sitting square.
const TWIST: f32 = 0.45;
/// Inward travel per tick (~120ms): the speed you fly down the tunnel.
const ZOOM_SPEED: f32 = 0.20;
/// Rotation per tick: the slow barrel-roll of the whole tunnel.
const SPIN_SPEED: f32 = 0.03;
/// Clamp near the center so the vanishing point doesn't alias into noise.
const MAX_DEPTH: f32 = 60.0;

/// One tunnel frame filling exactly `w`×`h` cells at `tick`.
pub fn frame(tick: u64, w: u16, h: u16, theme: &Theme) -> Text<'static> {
    let (w, h) = (w as usize, h as usize);
    if w == 0 || h == 0 {
        return Text::default();
    }

    let cx = (w as f32 - 1.0) / 2.0;
    let cy = (h as f32 - 1.0) / 2.0;
    let t = tick as f32;
    let zoom = t * ZOOM_SPEED;
    let spin = t * SPIN_SPEED;

    let mut lines = Vec::with_capacity(h);
    for y in 0..h {
        let dy = (y as f32 - cy) * ASPECT;

        // Coalesce equal-colored neighbours into one span so a line is a handful
        // of runs rather than `w` singletons.
        let mut spans: Vec<Span> = Vec::new();
        let mut run = String::new();
        let mut run_style = Style::default();
        let mut started = false;

        for x in 0..w {
            let dx = x as f32 - cx;
            let r = (dx * dx + dy * dy).sqrt().max(0.5);
            let depth = (DEPTH_SCALE / r).min(MAX_DEPTH);
            let angle = dy.atan2(dx);

            // Woven tunnel wall: concentric rings along depth times angular
            // spokes, the spokes twisting with depth into a spiral. The product
            // stays lively at the mouth (spokes still oscillate) instead of
            // flattening into an angular gradient.
            let rings = ((depth + zoom) * RING_FREQ).sin();
            let spokes = (angle * SPOKES + depth * TWIST + spin).cos();
            let v = 0.5 + 0.5 * rings * spokes;

            let ch = RAMP[(v * (RAMP.len() - 1) as f32).round() as usize] as char;
            let style = Style::default().fg(lerp(theme.dim, theme.accent, v));

            if started && style == run_style {
                run.push(ch);
            } else {
                if started {
                    spans.push(Span::styled(std::mem::take(&mut run), run_style));
                }
                run.push(ch);
                run_style = style;
                started = true;
            }
        }
        if started {
            spans.push(Span::styled(run, run_style));
        }
        lines.push(Line::from(spans));
    }
    Text::from(lines)
}

/// Linear blend between two RGB colors; non-RGB inputs snap to `b`.
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
            "consecutive ticks must move the tunnel"
        );
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
    fn zero_area_is_empty() {
        let th = Theme::preset(Accent::Amber);
        assert_eq!(frame(0, 0, 0, &th).lines.len(), 0);
    }
}
