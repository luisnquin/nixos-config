use crate::bg::dots::Dots;
use crate::bg::grid::Grid;
use crate::bg::Cell;

pub mod curtain;
pub mod flow;
pub mod hero;
pub mod lattice;
pub mod rings;
pub mod tunnel;

pub mod glitch;
pub mod skull;
pub mod surge;

#[derive(Clone, Copy)]
pub enum Effect {
    Cells(fn(f32, &mut Grid)),
    Dots(fn(f32, &mut Dots)),
    Shaded(fn(f32, usize, usize, &mut [Cell])),
}

pub struct Entry {
    pub name: &'static str,
    pub effect: Effect,
    pub speed: f32,
}

pub const ENTRIES: &[Entry] = &[
    Entry { name: "tunnel", effect: Effect::Shaded(tunnel::render), speed: 1.0 },
    Entry { name: "rings", effect: Effect::Dots(rings::render), speed: 1.0 },
    Entry { name: "lattice", effect: Effect::Cells(lattice::render), speed: 1.4 },
    Entry { name: "flow", effect: Effect::Dots(flow::render), speed: 0.6 },
    Entry { name: "curtain", effect: Effect::Cells(curtain::render), speed: 1.0 },
    Entry { name: "hero", effect: Effect::Dots(hero::render), speed: 2.4 },
];
