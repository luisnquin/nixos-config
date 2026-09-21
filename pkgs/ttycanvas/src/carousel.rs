use std::time::{SystemTime, UNIX_EPOCH};

use super::effects::{Entry, ENTRIES};

pub const SLOT: u64 = 200;

pub const DISSOLVE: u64 = 6;

const MAX_ENTRIES: usize = 8;
const _: () = assert!(ENTRIES.len() <= MAX_ENTRIES);
const _: () = assert!(!ENTRIES.is_empty());

#[derive(Clone, Copy)]
pub struct Carousel {
    seed: u64,
}

pub struct Slot {
    pub current: &'static Entry,
    pub incoming_from: Option<(&'static Entry, f32)>,
}

impl Carousel {
    pub fn from_clock() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Carousel::new(nanos)
    }

    pub fn new(seed: u64) -> Self {
        Carousel { seed }
    }

    pub fn at(&self, tick: u64) -> Slot {
        let slot = tick / SLOT;
        let current = &ENTRIES[self.pick(slot)];

        let phase = tick % SLOT;
        let incoming_from = if slot > 0 && phase < DISSOLVE {
            let prev = &ENTRIES[self.pick(slot - 1)];
            Some((prev, (phase + 1) as f32 / (DISSOLVE + 1) as f32))
        } else {
            None
        };

        Slot {
            current,
            incoming_from,
        }
    }

    fn pick(&self, slot: u64) -> usize {
        let n = ENTRIES.len();
        let cycle = slot / n as u64;
        let pos = (slot % n as u64) as usize;
        self.order(cycle)[pos]
    }

    fn order(&self, cycle: u64) -> [usize; MAX_ENTRIES] {
        let n = ENTRIES.len();
        let mut out = [0usize; MAX_ENTRIES];
        for (i, slot) in out.iter_mut().enumerate().take(n) {
            *slot = i;
        }
        let mut rng = Rng::new(self.seed ^ splitmix(cycle));
        for i in (1..n).rev() {
            out.swap(i, rng.below(i + 1));
        }
        out
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed })
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        let n = n as u64;
        let zone = u64::MAX - (u64::MAX % n) - 1;
        loop {
            let r = self.next();
            if r <= zone {
                return (r % n) as usize;
            }
        }
    }
}

fn splitmix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(c: &Carousel, cycle: u64) -> Vec<&'static str> {
        (0..ENTRIES.len() as u64)
            .map(|i| ENTRIES[c.pick(cycle * ENTRIES.len() as u64 + i)].name)
            .collect()
    }

    #[test]
    fn a_slot_boundary_changes_the_effect() {
        let c = Carousel::new(1234);
        let before = c.at(SLOT - 1).current.name;
        let after = c.at(SLOT).current.name;
        assert_ne!(before, after, "crossing a slot must change the effect");
    }

    #[test]
    fn a_slot_holds_one_effect_throughout() {
        let c = Carousel::new(99);
        let name = c.at(SLOT).current.name;
        for t in SLOT..SLOT * 2 {
            assert_eq!(c.at(t).current.name, name, "effect changed mid-slot at {t}");
        }
    }

    #[test]
    fn a_full_cycle_visits_every_effect_exactly_once() {
        for seed in [0u64, 1, 7, 42, 1 << 40, u64::MAX] {
            let c = Carousel::new(seed);
            for cycle in 0..4 {
                let mut seen = names(&c, cycle);
                seen.sort_unstable();
                let mut want: Vec<_> = ENTRIES.iter().map(|e| e.name).collect();
                want.sort_unstable();
                assert_eq!(seen, want, "seed {seed} cycle {cycle} is not a permutation");
            }
        }
    }

    #[test]
    fn different_seeds_give_different_orders() {
        let a = Carousel::new(0xfeed);
        let b = Carousel::new(0xbeef);
        let differs = (0..4).any(|c| names(&a, c) != names(&b, c));
        assert!(differs, "two seeds produced the same rotation");
    }

    #[test]
    fn a_cycle_reshuffles() {
        let c = Carousel::new(0x5eed);
        let differs = (0..8).any(|cycle| names(&c, cycle) != names(&c, cycle + 1));
        assert!(differs, "the order never changed across eight cycles");
    }

    #[test]
    fn the_dissolve_runs_only_at_a_slot_edge() {
        let c = Carousel::new(7);
        assert!(c.at(0).incoming_from.is_none(), "nothing precedes the first slot");
        for t in 0..DISSOLVE {
            assert!(c.at(SLOT + t).incoming_from.is_some(), "no dissolve at +{t}");
        }
        assert!(c.at(SLOT + DISSOLVE).incoming_from.is_none());
        let (_, p) = c.at(SLOT + DISSOLVE - 1).incoming_from.unwrap();
        assert!(p > 0.8, "dissolve ends at {p}, which leaves a visible step");
    }
}
