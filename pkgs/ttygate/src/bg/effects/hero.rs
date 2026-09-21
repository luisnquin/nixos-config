use super::{glitch, lattice, skull};
use crate::bg::dots::Dots;
use crate::bg::noise::{bayer, hash01};

const PITCH: usize = 4;

pub const DWELL: f32 = 60.0;

const BURST: f32 = 5.0;

const SCALE_MIN: f32 = 0.55;
const SCALE_SPAN: f32 = 0.45;

const YAW_BIAS: f32 = 0.16;

const APART: f32 = 1.0;

const TRIES: i32 = 10;
const CHAIN: i32 = 128;

pub fn render(tick: f32, d: &mut Dots) {
    let epoch = (tick / DWELL).floor();
    backdrop(tick, d);
    skull::render_at(tick, place(epoch as i32, d), d);
    let since = tick - epoch * DWELL;
    let heat = (1.0 - since / BURST).clamp(0.0, 1.0);
    glitch::apply(tick, heat * heat, d);
}

pub fn place(epoch: i32, d: &Dots) -> skull::Place {
    let anchor = epoch - epoch.rem_euclid(CHAIN);
    let mut at = pick(anchor, &draw(anchor - 1, 0, d), d);
    for e in anchor + 1..=epoch {
        at = pick(e, &at, d);
    }
    at
}

fn pick(epoch: i32, prev: &skull::Place, d: &Dots) -> skull::Place {
    let mut cand = draw(epoch, 0, d);
    let mut salt = 1;
    while salt < TRIES && !apart(prev, &cand) {
        cand = draw(epoch, salt, d);
        salt += 1;
    }
    cand
}

fn apart(a: &skull::Place, b: &skull::Place) -> bool {
    let s = a.scale.max(b.scale);
    (b.dx - a.dx).abs() >= APART * skull::HALF_W * s
        || (b.dy - a.dy).abs() >= APART * skull::HALF_H * s
}

fn draw(epoch: i32, salt: i32, d: &Dots) -> skull::Place {
    let key = salt * 4;
    let scale = SCALE_MIN + SCALE_SPAN * hash01(epoch, key + 1);
    let half_w = if d.dh == 0 {
        0.0
    } else {
        d.dw as f32 / d.dh as f32
    };
    let room_x = (half_w - skull::HALF_W * scale).max(0.0);
    let room_y = (1.0 - skull::HALF_H * scale).max(0.0);
    skull::Place {
        dx: (hash01(epoch, key + 2) - 0.5) * 2.0 * room_x,
        dy: (hash01(epoch, key + 3) - 0.5) * 2.0 * room_y,
        scale,
        yaw: (hash01(epoch, key + 4) - 0.5) * 2.0 * YAW_BIAS,
    }
}

fn backdrop(tick: f32, d: &mut Dots) {
    let p = lattice::phases(tick);
    for y in (0..d.dh).step_by(PITCH) {
        let cy = y as f32 * 0.25;
        for x in (0..d.dw).step_by(PITCH) {
            if lattice::wave(x as f32 * 0.5, cy, &p) > bayer(x / PITCH, y / PITCH) {
                d.set(x, y);
            }
        }
    }
}
