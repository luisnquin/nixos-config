use super::surge;
use crate::bg::grid::Grid;
use crate::bg::noise::{bayer, value_noise};

pub const PITCH_X: usize = 3;
pub const PITCH_Y: usize = 2;

const WAVE: f32 = 0.16;
pub const DRIFT: f32 = 0.20;
pub const BLOTCH_DRIFT: f32 = 0.004;
const GAIN: f32 = 1.15;

pub const SURGE: surge::Surge = surge::Surge { period: 470.0 };
pub const DRIFT_SURGE: f32 = 2.0;
pub const BLOTCH_SURGE: f32 = 2.0;

const GAIN_SURGE: f32 = 0.55;

pub struct Phases {
    drift: f32,
    blotch: f32,
    gain: f32,
}

pub fn phases(tick: f32) -> Phases {
    Phases {
        drift: SURGE.phase(DRIFT, DRIFT_SURGE, tick),
        blotch: SURGE.phase(BLOTCH_DRIFT, BLOTCH_SURGE, tick),
        gain: GAIN + GAIN_SURGE * SURGE.level(tick),
    }
}

pub fn wave(x: f32, y: f32, p: &Phases) -> f32 {
    let sweep = (x + 2.0 * y) * WAVE - p.drift;
    let band = 0.5 + 0.5 * sweep.sin();
    let blotch = 0.78 + 0.32 * value_noise(x * 0.035, y * 0.07 + p.blotch);
    band.powi(3) * blotch * p.gain
}

pub fn render(tick: f32, g: &mut Grid) {
    let p = phases(tick);
    for y in (0..g.h).step_by(PITCH_Y) {
        for x in (0..g.w).step_by(PITCH_X) {
            if wave(x as f32, y as f32, &p) > bayer(x / PITCH_X, y / PITCH_Y) {
                g.set(x, y, b'.');
            }
        }
    }
}
