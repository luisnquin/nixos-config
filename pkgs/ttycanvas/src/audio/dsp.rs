use std::f32::consts::TAU;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }

    pub fn roll(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn unit(&mut self) -> f32 {
        (self.roll() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn bipolar(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

#[derive(Default)]
pub struct Phasor {
    pub phase: f32,
}

impl Phasor {
    pub fn step(&mut self, hz: f32, rate: f32) -> f32 {
        let p = self.phase;
        self.phase += hz / rate;
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }
        p
    }
}

pub fn sine(p: f32) -> f32 {
    (p * TAU).sin()
}

pub fn saw(p: f32) -> f32 {
    2.0 * p - 1.0
}

pub fn square(p: f32) -> f32 {
    if p < 0.5 { 1.0 } else { -1.0 }
}

pub fn coef(hz: f32, rate: f32) -> f32 {
    1.0 - (-TAU * hz / rate).exp()
}

#[derive(Default)]
pub struct OnePole {
    z: f32,
}

impl OnePole {
    pub fn low(&mut self, x: f32, k: f32) -> f32 {
        self.z += k * (x - self.z);
        self.z
    }

    pub fn high(&mut self, x: f32, k: f32) -> f32 {
        x - self.low(x, k)
    }
}

#[derive(Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

pub struct Bands {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

impl Svf {
    pub fn run(&mut self, x: f32, hz: f32, q: f32, rate: f32) -> Bands {
        let g = (std::f32::consts::PI * (hz / rate).clamp(0.0001, 0.49)).tan();
        let k = 1.0 / q.max(0.1);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        Bands {
            low: v2,
            band: v1,
            high: x - k * v1 - v2,
        }
    }
}

#[derive(Default)]
pub struct Slew {
    pub value: f32,
}

impl Slew {
    pub fn run(&mut self, target: f32, up: f32, down: f32) -> f32 {
        let k = if target > self.value { up } else { down };
        self.value += k * (target - self.value);
        self.value
    }
}

#[derive(Default)]
pub struct Decay {
    pub value: f32,
}

impl Decay {
    pub fn fire(&mut self) {
        self.value = 1.0;
    }

    pub fn run(&mut self, k: f32) -> f32 {
        let v = self.value;
        self.value *= 1.0 - k;
        v
    }

    pub fn live(&self) -> bool {
        self.value > 0.0005
    }
}

#[derive(Default)]
pub struct Crusher {
    held: f32,
    count: f32,
}

impl Crusher {
    pub fn run(&mut self, x: f32, hold: f32, bits: f32) -> f32 {
        self.count += 1.0;
        if self.count >= hold {
            self.count = 0.0;
            let steps = 2f32.powf(bits.clamp(2.0, 16.0));
            self.held = (x * steps).round() / steps;
        }
        self.held
    }
}

pub struct Stutter {
    buf: Vec<f32>,
    len: usize,
    pos: usize,
    filling: bool,
}

impl Stutter {
    pub fn new(max: usize) -> Self {
        Stutter {
            buf: vec![0.0; max.max(1)],
            len: 0,
            pos: 0,
            filling: false,
        }
    }

    pub fn cut(&mut self, len: usize) {
        self.len = len.clamp(1, self.buf.len());
        self.pos = 0;
        self.filling = true;
    }

    pub fn release(&mut self) {
        self.len = 0;
    }

    pub fn engaged(&self) -> bool {
        self.len > 0
    }

    pub fn run(&mut self, x: f32) -> f32 {
        if self.len == 0 {
            return x;
        }
        let out = if self.filling {
            self.buf[self.pos] = x;
            x
        } else {
            self.buf[self.pos]
        };
        self.pos += 1;
        if self.pos >= self.len {
            self.pos = 0;
            self.filling = false;
        }
        out
    }
}

pub fn soft(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

pub fn hard(x: f32, drive: f32) -> f32 {
    (x * drive).clamp(-1.0, 1.0)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stutter_repeats_its_first_slice() {
        let mut s = Stutter::new(64);
        s.cut(4);
        let first: Vec<f32> = (0..4).map(|i| s.run(i as f32)).collect();
        assert_eq!(first, vec![0.0, 1.0, 2.0, 3.0]);
        let again: Vec<f32> = (0..4).map(|i| s.run(10.0 + i as f32)).collect();
        assert_eq!(again, first, "the loop did not repeat the captured slice");
        s.release();
        assert_eq!(s.run(7.0), 7.0);
    }

    #[test]
    fn the_crusher_holds_and_quantises() {
        let mut c = Crusher::default();
        let a = c.run(0.31, 3.0, 3.0);
        let b = c.run(0.9, 3.0, 3.0);
        assert_eq!(a, b, "value changed before the hold ran out");
        assert!((a * 8.0).fract().abs() < 1e-5, "not on a 3-bit grid: {a}");
    }

    #[test]
    fn the_svf_separates_bands() {
        let mut f = Svf::default();
        let rate = 48000.0;
        let mut low_energy = 0.0;
        let mut high_energy = 0.0;
        let mut p = Phasor::default();
        for _ in 0..4800 {
            let x = sine(p.step(50.0, rate));
            let b = f.run(x, 1000.0, 0.7, rate);
            low_energy += b.low * b.low;
            high_energy += b.high * b.high;
        }
        assert!(
            low_energy > high_energy * 50.0,
            "a 50 Hz tone leaked into the high band"
        );
    }
}
