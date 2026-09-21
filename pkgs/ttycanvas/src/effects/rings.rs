use super::surge;
use crate::dots::Dots;
use crate::field::Field;
use crate::noise::hash01;

const DOT: f32 = 0.5;
const DEPTH_K: f32 = 150.0;
const HALF_W: f32 = 1.5;
pub const ZOOM: f32 = 0.030;
pub const SPIN: f32 = 0.035;

pub const SURGE: surge::Surge = surge::Surge { period: 730.0 };
pub const ZOOM_SURGE: f32 = 2.4;
pub const SPIN_SURGE: f32 = 1.6;

const HALF_W_SURGE: f32 = 1.1;
const CORE_DEPTH: f32 = 6.6;

pub fn render(tick: f32, d: &mut Dots) {
    let mut f = Field::dots(d);
    band(tick, &mut f);
    f.dither_dots(d, 0.0);
}

fn band(tick: f32, f: &mut Field) {
    if f.w == 0 || f.h == 0 {
        return;
    }
    let cx = (f.w as f32 - 1.0) * 0.5;
    let cy = (f.h as f32 - 1.0) * 0.5;
    let zoom = SURGE.phase(ZOOM, ZOOM_SURGE, tick);
    let spin = SURGE.phase(SPIN, SPIN_SURGE, tick);
    let half_w = HALF_W + HALF_W_SURGE * SURGE.level(tick);

    for y in 0..f.h {
        let dy = (y as f32 - cy) * DOT;
        for x in 0..f.w {
            let dx = (x as f32 - cx) * DOT;
            let r = (dx * dx + dy * dy).sqrt().max(0.75);
            let depth = DEPTH_K / r;

            let phase = depth - zoom;
            let off = phase - phase.floor() - 0.5;
            let dr = off * r * r / DEPTH_K;
            let t = dr.abs() / half_w;
            if t >= 1.0 {
                continue;
            }
            let ring = 1.0 - t * t * t * t;

            let angle = dy.atan2(dx);
            let sweep = 0.93 + 0.07 * (angle * 2.0 - spin).cos();
            let core = 1.0 / (1.0 + (depth / CORE_DEPTH).powi(8));
            let grain = 0.86 + 0.14 * hash01(x as i32, y as i32);

            f.set(x, y, ring * sweep * core * grain);
        }
    }
}
