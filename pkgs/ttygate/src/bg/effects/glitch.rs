use crate::bg::dots::Dots;
use crate::bg::noise::hash01;

const GROUP: i32 = 4;
const EPOCH: f32 = 1.5;
const QUIET: f32 = 0.84;
const SHIFT: f32 = 36.0;
const DROP: f32 = 0.10;
const SPECK: f32 = 0.02;

const BURST_QUIET: f32 = 0.30;
const BURST_DROP: f32 = 0.30;
const BURST_SPECK: f32 = 0.07;

pub fn apply(tick: f32, heat: f32, d: &mut Dots) {
    if d.dw == 0 || d.dh == 0 {
        return;
    }
    let heat = heat.clamp(0.0, 1.0);
    let quiet = QUIET + (BURST_QUIET - QUIET) * heat;
    let drop = DROP + (BURST_DROP - DROP) * heat;
    let speck = SPECK + (BURST_SPECK - SPECK) * heat;
    let src = d.buf.clone();
    let epoch = (tick * EPOCH) as i32;
    let rows = d.dh as i32;

    for x in 0..d.dw {
        let group = x as i32 / GROUP;
        let roll = hash01(group, epoch);
        if roll < quiet {
            continue;
        }
        let strength = (roll - quiet) / (1.0 - quiet);
        let dy = ((hash01(group, epoch + 77) - 0.5) * 2.0 * SHIFT * strength).round() as i32;

        for y in 0..d.dh {
            let sy = (y as i32 + dy).rem_euclid(rows) as usize;
            let mut on = src[sy * d.dw + x];
            let rate = if on { drop } else { speck };
            if hash01(x as i32 * 31 + 5, y as i32 + epoch * 101) < rate {
                on = !on;
            }
            d.put(x, y, on);
        }
    }
}
