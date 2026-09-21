//! The VT console font has nothing in U+2800..28FF, so the braille backends
//! fold each 2x4 dot block to its lit-dot count and shade it through `RAMP`.

pub mod carousel;
pub mod dots;
pub mod effects;
pub mod field;
pub mod grid;
pub mod noise;

use dots::Dots;
use grid::Grid;

pub const RAMP: &[u8] = b" .:-=+*#%@";

#[derive(Clone, Copy, PartialEq)]
pub struct Cell {
    pub ch: char,
    pub v: f32,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ch: ' ', v: 0.0 }
    }
}

pub fn render(effect: effects::Effect, tick: f32, w: usize, h: usize) -> Vec<Cell> {
    let mut out = vec![Cell::default(); w * h];
    if w == 0 || h == 0 {
        return out;
    }
    match effect {
        effects::Effect::Cells(f) => {
            let mut g = Grid::new(w, h);
            f(tick, &mut g);
            for (i, c) in out.iter_mut().enumerate() {
                *c = cell_of(g.buf[i]);
            }
        }
        effects::Effect::Dots(f) => {
            let mut d = Dots::new(w, h);
            f(tick, &mut d);
            for y in 0..h {
                for x in 0..w {
                    out[y * w + x] = fold(d.mask(x, y));
                }
            }
        }
        effects::Effect::Shaded(f) => f(tick, w, h, &mut out),
    }
    out
}

fn fold(mask: u8) -> Cell {
    let v = mask.count_ones() as f32 / 8.0;
    Cell {
        ch: RAMP[(v * (RAMP.len() - 1) as f32).round() as usize] as char,
        v,
    }
}

fn cell_of(b: u8) -> Cell {
    match RAMP.iter().position(|&r| r == b) {
        Some(i) => Cell {
            ch: b as char,
            v: i as f32 / (RAMP.len() - 1) as f32,
        },
        None => Cell {
            ch: b as char,
            v: 0.22,
        },
    }
}
