use super::surge;
use crate::dots::Dots;
use crate::noise::{bayer, fbm, value_noise};

const SCALE: f32 = 0.011;
pub const DRIFT_X: f32 = 0.018;
pub const DRIFT_Y: f32 = 0.006;
const CONTOURS: f32 = 13.0;
pub const CONTOUR_DRIFT: f32 = 0.05;
pub const CLUMP_DRIFT: f32 = 0.02;
const HALF_W: f32 = 4.0;
const MAX_COVER: f32 = 0.17;

pub const SURGE: surge::Surge = surge::Surge { period: 550.0 };
pub const DRIFT_SURGE: f32 = 2.6;
pub const CONTOUR_SURGE: f32 = 3.4;
pub const CLUMP_SURGE: f32 = 2.6;

const WIDEN_SURGE: f32 = 0.45;
const OCTAVES: u32 = 3;

pub fn render(tick: f32, d: &mut Dots) {
    let field = |px: f32, py: f32| fbm(px, py, OCTAVES) * CONTOURS;

    let drift_x = SURGE.phase(DRIFT_X, DRIFT_SURGE, tick);
    let drift_y = SURGE.phase(DRIFT_Y, DRIFT_SURGE, tick);
    let slide = SURGE.phase(CONTOUR_DRIFT, CONTOUR_SURGE, tick);
    let clump_t = SURGE.phase(CLUMP_DRIFT, CLUMP_SURGE, tick);
    let widen = 1.0 + WIDEN_SURGE * SURGE.level(tick);

    for y in 0..d.dh {
        let fy = y as f32 * SCALE - drift_y;
        for x in 0..d.dw {
            let fx = x as f32 * SCALE + drift_x;

            let f = field(fx, fy);
            let gx = field(fx + SCALE, fy) - f;
            let gy = field(fx, fy + SCALE) - f;
            let grad = (gx * gx + gy * gy).sqrt().max(1e-4);

            let phase = f - slide;
            let off = phase - phase.floor() - 0.5;
            let dist = off / grad;
            let half = HALF_W.min(MAX_COVER / grad) * widen;
            let t = dist.abs() / half;
            if t >= 1.0 {
                continue;
            }
            let ribbon = 1.0 - t * t * t * t;

            let clump = value_noise(x as f32 * 0.15, y as f32 * 0.15 + clump_t);
            let clump = 0.55 + 0.70 * clump;

            if ribbon * clump > bayer(x, y) {
                d.set(x, y);
            }
        }
    }
}
