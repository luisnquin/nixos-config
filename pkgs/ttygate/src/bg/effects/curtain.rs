use super::surge;
use crate::bg::grid::{ramp_pick, Grid, RAMP_FULL};
use crate::bg::noise::{bayer, hash01};

const STRIP: i32 = 3;
pub const FLICKER: f32 = 0.22;
const GATE: f32 = 0.30;
const GATE_SOFT: f32 = 0.18;
pub const SCROLL_MIN: f32 = 0.6;
pub const SCROLL_SPAN: f32 = 3.2;

pub const SURGE: surge::Surge = surge::Surge { period: 610.0 };
pub const SCROLL_SURGE: f32 = 2.2;
pub const FLICKER_SURGE: f32 = 1.7;

const GATE_SURGE: f32 = 0.12;
const BAND_FREQ: f32 = 0.20;
const EDGE_FADE: f32 = 5.0;
const SPECKS: &[u8] = b"/\\|-_=+:;7";

pub fn render(tick: f32, g: &mut Grid) {
    if g.w == 0 || g.h == 0 {
        return;
    }
    let cx = (g.w as f32 - 1.0) * 0.5;
    let half = (g.w as f32 * 0.5).max(1.0);
    let rows = g.h as f32;
    let epoch = SURGE.phase(FLICKER, FLICKER_SURGE, tick) as i32;
    let gate = GATE - GATE_SURGE * SURGE.level(tick);
    let scroll_t = tick + SCROLL_SURGE * SURGE.integral(tick);

    for x in 0..g.w {
        let col = x as i32;
        let strip = col / STRIP;

        let energy = 0.35 * hash01(strip, 0) + 0.65 * hash01(strip, 1 + epoch);
        let centre = (1.0 - ((x as f32 - cx) / half).abs() * 0.9).max(0.0);
        let drive = energy * (0.45 + 0.55 * centre);
        let open = ((drive - gate) / GATE_SOFT).clamp(0.0, 1.0);

        let scroll =
            hash01(strip, 9) * 64.0 + scroll_t * (SCROLL_MIN + SCROLL_SPAN * hash01(strip, 5));
        let top = hash01(strip, 3) * rows * 0.30;
        let bottom = rows - hash01(strip, 4) * rows * 0.28;

        for y in 0..g.h {
            if open > 0.0 {
                let fy = y as f32;
                let edge = (((fy - top) / EDGE_FADE).clamp(0.0, 1.0)
                    * ((bottom - fy) / EDGE_FADE).clamp(0.0, 1.0))
                    .max(0.0);
                if edge > 0.0 {
                    let yy = fy + scroll;
                    let bands = 0.55 + 0.45 * (yy * BAND_FREQ + strip as f32 * 0.7).sin();
                    let grain = hash01(col, yy as i32);

                    let v = open * edge * bands * (0.45 + 0.80 * grain);
                    let ch = ramp_pick(RAMP_FULL, v + (bayer(x, y) - 0.5) * 0.16);
                    if ch != b' ' {
                        g.set(x, y, ch);
                        continue;
                    }
                }
            }
            if hash01(col * 31, y as i32 + epoch * 13) > 0.9965 {
                let i = (hash01(col, y as i32) * SPECKS.len() as f32) as usize;
                g.set(x, y, SPECKS[i % SPECKS.len()]);
            }
        }
    }
}
