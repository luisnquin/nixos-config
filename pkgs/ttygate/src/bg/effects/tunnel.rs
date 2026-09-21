use crate::bg::{Cell, RAMP};

const ASPECT: f32 = 2.0;
const DEPTH_SCALE: f32 = 22.0;
const RING_FREQ: f32 = 1.35;
const SPOKES: f32 = 7.0;
const TWIST: f32 = 0.45;
const ZOOM_SPEED: f32 = 0.20;
const SPIN_SPEED: f32 = 0.03;
const MAX_DEPTH: f32 = 60.0;

pub fn render(tick: f32, w: usize, h: usize, out: &mut [Cell]) {
    let cx = (w as f32 - 1.0) / 2.0;
    let cy = (h as f32 - 1.0) / 2.0;
    let zoom = tick * ZOOM_SPEED;
    let spin = tick * SPIN_SPEED;

    for y in 0..h {
        let dy = (y as f32 - cy) * ASPECT;
        for x in 0..w {
            let dx = x as f32 - cx;
            let r = (dx * dx + dy * dy).sqrt().max(0.5);
            let depth = (DEPTH_SCALE / r).min(MAX_DEPTH);
            let angle = dy.atan2(dx);

            let rings = ((depth + zoom) * RING_FREQ).sin();
            let spokes = (angle * SPOKES + depth * TWIST + spin).cos();
            let v = 0.5 + 0.5 * rings * spokes;

            out[y * w + x] = Cell {
                ch: RAMP[(v * (RAMP.len() - 1) as f32).round() as usize] as char,
                v,
            };
        }
    }
}
