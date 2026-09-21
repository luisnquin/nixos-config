use std::f32::consts::TAU;

use super::Cue;
use super::dsp::{
    Crusher, Decay, Phasor, Rng, Slew, Stutter, Svf, coef, hard, lerp, saw, sine, soft, square,
};
use crate::carousel::{Carousel, DISSOLVE};
use crate::effects::{Entry, curtain, flow, hero, lattice, rings, tunnel};
use crate::noise::value_noise;

const BED_GAIN: f32 = 0.9;
const HEAT_ON: f32 = 0.12;
const AHEAD: f64 = 1.25;

#[derive(Clone, Copy, Default)]
pub struct Bed {
    drone_hz: f32,
    drone_shape: f32,
    drone_cut: f32,
    drone_level: f32,
    drone_grit: f32,
    sub_level: f32,
    ring_hz: f32,
    ring_level: f32,
    wind_level: f32,
    wind_cut: f32,
    wind_q: f32,
    pulse_bpm: f32,
    pulse_level: f32,
    chatter_rate: f32,
    chatter_level: f32,
    crackle_rate: f32,
    crackle_level: f32,
    heat: f32,
}

macro_rules! lerp_fields {
    ($a:expr, $b:expr, $t:expr; $($f:ident),*) => {
        Bed { $($f: lerp($a.$f, $b.$f, $t)),* }
    };
}

fn mix_bed(a: &Bed, b: &Bed, t: f32) -> Bed {
    lerp_fields!(a, b, t;
        drone_hz, drone_shape, drone_cut, drone_level, drone_grit, sub_level,
        ring_hz, ring_level, wind_level, wind_cut, wind_q, pulse_bpm, pulse_level,
        chatter_rate, chatter_level, crackle_rate, crackle_level, heat)
}

pub fn bed(name: &str, t: f32) -> Bed {
    match name {
        "tunnel" => {
            let spin = (t * tunnel::SPIN_SPEED).sin();
            Bed {
                drone_hz: 55.0,
                drone_shape: 0.0,
                drone_cut: 170.0 + 90.0 * spin,
                drone_level: 0.30,
                drone_grit: 0.2,
                sub_level: 0.28,
                ring_hz: 3200.0,
                ring_level: 0.03,
                wind_level: 0.10,
                wind_cut: 350.0,
                wind_q: 1.0,
                crackle_rate: 3.0,
                crackle_level: 0.15,
                ..Bed::default()
            }
        }
        "rings" => {
            let lvl = rings::SURGE.level(t);
            Bed {
                drone_hz: 41.2,
                drone_shape: 0.3,
                drone_cut: 120.0 + 420.0 * lvl,
                drone_level: 0.22,
                drone_grit: 0.1,
                sub_level: 0.30,
                ring_hz: 4000.0,
                ring_level: 0.04 + 0.05 * lvl,
                wind_level: 0.06,
                wind_cut: 800.0,
                wind_q: 1.2,
                ..Bed::default()
            }
        }
        "lattice" => {
            let lvl = lattice::SURGE.level(t);
            Bed {
                drone_hz: 65.4,
                drone_shape: 0.8,
                drone_cut: 260.0,
                drone_level: 0.16,
                drone_grit: 0.3,
                sub_level: 0.10,
                ring_hz: 2800.0,
                ring_level: 0.03,
                wind_level: 0.04,
                wind_cut: 1500.0,
                wind_q: 3.0,
                chatter_rate: 7.0 * (1.0 + lattice::DRIFT_SURGE * lvl),
                chatter_level: 0.35,
                crackle_rate: 2.0,
                crackle_level: 0.10,
                ..Bed::default()
            }
        }
        "flow" => {
            let lvl = flow::SURGE.level(t);
            let wander = value_noise(t * 0.03, 7.5);
            Bed {
                drone_hz: 49.0,
                drone_shape: 0.0,
                drone_cut: 140.0,
                drone_level: 0.14,
                drone_grit: 0.0,
                sub_level: 0.20,
                ring_hz: 3520.0,
                ring_level: 0.07 + 0.06 * lvl,
                wind_level: 0.35,
                wind_cut: 250.0 + 700.0 * lvl + 400.0 * wander,
                wind_q: 2.2,
                crackle_rate: 0.5,
                crackle_level: 0.10,
                ..Bed::default()
            }
        }
        "curtain" => {
            let lvl = curtain::SURGE.level(t);
            Bed {
                drone_hz: 73.4,
                drone_shape: 0.2,
                drone_cut: 220.0,
                drone_level: 0.20,
                drone_grit: 0.45,
                sub_level: 0.15,
                ring_hz: 3000.0,
                ring_level: 0.03,
                wind_level: 0.12,
                wind_cut: 2400.0,
                wind_q: 1.5,
                chatter_rate: 3.0,
                chatter_level: 0.15,
                crackle_rate: 25.0 + 40.0 * lvl,
                crackle_level: 0.30,
                ..Bed::default()
            }
        }
        "hero" => {
            let heat = hero::heat(t);
            Bed {
                drone_hz: 36.7,
                drone_shape: 0.5,
                drone_cut: 110.0 + 200.0 * heat,
                drone_level: 0.24,
                drone_grit: 0.6,
                sub_level: 0.40,
                ring_hz: 3800.0,
                ring_level: 0.05 + 0.35 * heat,
                wind_level: 0.08,
                wind_cut: 500.0,
                wind_q: 1.0,
                pulse_bpm: 54.0,
                pulse_level: 0.70,
                chatter_rate: 2.0,
                chatter_level: 0.15,
                crackle_rate: 4.0,
                crackle_level: 0.15,
                heat,
            }
        }
        _ => Bed {
            drone_hz: 55.0,
            drone_cut: 200.0,
            drone_level: 0.2,
            sub_level: 0.2,
            ring_hz: 3200.0,
            ring_level: 0.03,
            wind_level: 0.1,
            wind_cut: 600.0,
            wind_q: 1.0,
            ..Bed::default()
        },
    }
}

pub fn beat(name: &str, t: f32) -> i64 {
    match name {
        "tunnel" => (t * tunnel::ZOOM_SPEED * tunnel::RING_FREQ / TAU).floor() as i64,
        "rings" => rings::SURGE
            .phase(rings::ZOOM, rings::ZOOM_SURGE, t)
            .floor() as i64,
        "curtain" => curtain::SURGE.phase(curtain::FLICKER, curtain::FLICKER_SURGE, t) as i64,
        "hero" => (t / hero::DWELL).floor() as i64,
        _ => 0,
    }
}

#[derive(Clone, Copy)]
enum Shot {
    Blip {
        hz: f32,
        ms: f32,
        crush: f32,
    },
    Kick {
        hz: f32,
        ms: f32,
        level: f32,
    },
    Ping,
    Whoomp,
    Crackle,
    Spike,
    Sweep {
        from: f32,
        to: f32,
        ms: f32,
        level: f32,
    },
    Growl,
    Chord,
    Heat(f32),
}

#[derive(Default)]
struct Drone {
    a: Phasor,
    b: Phasor,
    sub: Phasor,
    filter: Svf,
    cut: Slew,
    level: Slew,
}

#[derive(Default)]
struct Ring {
    osc: Phasor,
    vib: Phasor,
    trem: Phasor,
    level: Slew,
    spike: Decay,
}

#[derive(Default)]
struct Wind {
    filter: Svf,
    cut: Slew,
    level: Slew,
}

#[derive(Default)]
struct Pulse {
    clock: Phasor,
    beat: Phasor,
    env: Decay,
    level: Slew,
    was: f32,
}

#[derive(Default)]
struct Blip {
    osc: Phasor,
    hz: f32,
    env: Decay,
    k: f32,
    crush: f32,
    crusher: Crusher,
}

#[derive(Default)]
struct Kick {
    osc: Phasor,
    env: Decay,
    hz: f32,
    k: f32,
    level: f32,
}

#[derive(Default)]
struct Sweep {
    env: Decay,
    from: f32,
    to: f32,
    k: f32,
    level: f32,
    osc: Phasor,
    filter: Svf,
}

#[derive(Default)]
struct Growl {
    carrier: Phasor,
    mod_: Phasor,
    env: Decay,
    k: f32,
}

#[derive(Default)]
struct Chord {
    osc: [Phasor; 3],
    env: Slew,
    gate: f32,
    age: u32,
}

struct Glitch {
    stutter: Stutter,
    crusher: Crusher,
    scream: Svf,
    recut: u32,
    heat: Slew,
    pan: f32,
}

pub struct Engine {
    rate: f32,
    tick_len: f64,
    carousel: Carousel,
    tick: f64,
    anchor: f64,
    last_slot: Option<u64>,
    rng: Rng,
    volume: f32,
    fade: Slew,
    fading: bool,
    fade_wait: u32,
    drone: Drone,
    ring: Ring,
    wind: Wind,
    pulse: Pulse,
    blips: [Blip; 6],
    next_blip: usize,
    kick: Kick,
    ping: Blip,
    sweep: Sweep,
    growl: Growl,
    chord: Chord,
    glitch: Glitch,
    stinger: Decay,
    stinger_k: f32,
    scan: Slew,
    scan_on: bool,
    scan_clock: Phasor,
    crackle_burst: Decay,
    queue: Vec<(u32, Shot)>,
    prev_t: Option<(&'static str, f32)>,
}

impl Engine {
    pub fn new(rate: f32, tick_ms: u64, carousel: Carousel, volume: f32) -> Self {
        let tick_len = rate as f64 * tick_ms.max(1) as f64 / 1000.0;
        let stinger_k = coef(
            1000.0 / (tick_ms.max(1) as f32 * DISSOLVE as f32) * 2.5,
            rate,
        );
        Engine {
            rate,
            tick_len,
            carousel,
            tick: 0.0,
            anchor: 0.0,
            last_slot: None,
            rng: Rng::new(0xc0ffee),
            volume: volume.clamp(0.0, 1.0),
            fade: Slew { value: 0.0 },
            fading: false,
            fade_wait: 0,
            drone: Drone::default(),
            ring: Ring::default(),
            wind: Wind::default(),
            pulse: Pulse::default(),
            blips: Default::default(),
            next_blip: 0,
            kick: Kick::default(),
            ping: Blip::default(),
            sweep: Sweep::default(),
            growl: Growl::default(),
            chord: Chord::default(),
            glitch: Glitch {
                stutter: Stutter::new((rate * 0.08) as usize),
                crusher: Crusher::default(),
                scream: Svf::default(),
                recut: 0,
                heat: Slew::default(),
                pan: 0.0,
            },
            stinger: Decay::default(),
            stinger_k,
            scan: Slew::default(),
            scan_on: false,
            scan_clock: Phasor::default(),
            crackle_burst: Decay::default(),
            queue: Vec::new(),
            prev_t: None,
        }
    }

    pub fn sync(&mut self, tick: u64) {
        let t = tick as f64;
        self.anchor = t;
        if t > self.tick || self.tick - t > AHEAD {
            self.tick = t;
        }
    }

    pub fn cue(&mut self, cue: Cue) {
        let ms = |x: f32| x;
        match cue {
            Cue::Key => {
                let hz = 700.0 + 900.0 * self.rng.unit();
                self.fire(Shot::Blip {
                    hz,
                    ms: ms(38.0),
                    crush: 0.7,
                });
            }
            Cue::Erase => {
                self.fire(Shot::Blip {
                    hz: 240.0,
                    ms: 70.0,
                    crush: 0.5,
                });
            }
            Cue::Nav => {
                self.fire(Shot::Blip {
                    hz: 1100.0,
                    ms: 30.0,
                    crush: 0.4,
                });
                self.later(
                    40.0,
                    Shot::Blip {
                        hz: 1500.0,
                        ms: 40.0,
                        crush: 0.4,
                    },
                );
            }
            Cue::Submit => {
                self.fire(Shot::Sweep {
                    from: 200.0,
                    to: 4200.0,
                    ms: 480.0,
                    level: 0.55,
                });
                self.fire(Shot::Kick {
                    hz: 70.0,
                    ms: 160.0,
                    level: 0.5,
                });
            }
            Cue::Scan(on) => self.scan_on = on,
            Cue::Fail => {
                self.fire(Shot::Heat(1.0));
                self.fire(Shot::Growl);
                self.fire(Shot::Spike);
                self.fire(Shot::Kick {
                    hz: 55.0,
                    ms: 220.0,
                    level: 0.8,
                });
                for (i, hz) in [900.0, 600.0, 300.0].into_iter().enumerate() {
                    self.later(
                        i as f32 * 130.0,
                        Shot::Blip {
                            hz,
                            ms: 90.0,
                            crush: 0.9,
                        },
                    );
                }
            }
            Cue::Grant => {
                self.scan_on = false;
                self.fire(Shot::Chord);
                self.fire(Shot::Sweep {
                    from: 300.0,
                    to: 9000.0,
                    ms: 700.0,
                    level: 0.25,
                });
                self.later(
                    60.0,
                    Shot::Kick {
                        hz: 60.0,
                        ms: 300.0,
                        level: 0.6,
                    },
                );
            }
            Cue::Cancel => {
                self.scan_on = false;
                self.fire(Shot::Sweep {
                    from: 1200.0,
                    to: 80.0,
                    ms: 350.0,
                    level: 0.5,
                });
                self.fire(Shot::Heat(0.45));
            }
            Cue::Fade => {
                if !self.fading && self.fade_wait == 0 {
                    self.fade_wait = (self.rate * 0.45) as u32;
                }
            }
        }
    }

    pub fn finished(&self) -> bool {
        self.fading && self.fade.value < 0.001
    }

    fn fire(&mut self, shot: Shot) {
        self.queue.push((0, shot));
    }

    fn later(&mut self, ms: f32, shot: Shot) {
        self.queue.push(((ms / 1000.0 * self.rate) as u32, shot));
    }

    fn trigger(&mut self, shot: Shot) {
        let rate = self.rate;
        match shot {
            Shot::Blip { hz, ms, crush } => {
                let slot = self.next_blip;
                self.next_blip = (slot + 1) % self.blips.len();
                let b = &mut self.blips[slot];
                b.hz = hz;
                b.k = coef(1000.0 / ms, rate) * 1.5;
                b.crush = crush;
                b.env.fire();
            }
            Shot::Kick { hz, ms, level } => {
                self.kick.hz = hz;
                self.kick.k = coef(1000.0 / ms, rate);
                self.kick.level = level;
                self.kick.osc.phase = 0.0;
                self.kick.env.fire();
            }
            Shot::Ping => {
                self.ping.hz = 1760.0;
                self.ping.k = coef(3.5, rate);
                self.ping.crush = 0.0;
                self.ping.env.fire();
                self.later(
                    160.0,
                    Shot::Blip {
                        hz: 1760.0,
                        ms: 220.0,
                        crush: 0.0,
                    },
                );
            }
            Shot::Whoomp => {
                self.kick.hz = 48.0;
                self.kick.k = coef(2.6, rate);
                self.kick.level = 0.55;
                self.kick.env.fire();
            }
            Shot::Crackle => self.crackle_burst.fire(),
            Shot::Spike => self.ring.spike.fire(),
            Shot::Sweep {
                from,
                to,
                ms,
                level,
            } => {
                self.sweep.from = from;
                self.sweep.to = to;
                self.sweep.k = coef(1000.0 / ms, rate) * 0.8;
                self.sweep.level = level;
                self.sweep.env.fire();
            }
            Shot::Growl => {
                self.growl.k = coef(1.6, rate);
                self.growl.env.fire();
            }
            Shot::Chord => {
                self.chord.gate = 1.0;
                self.chord.age = 0;
            }
            Shot::Heat(h) => {
                self.stinger.value = self.stinger.value.max(h);
                self.glitch.pan = self.rng.bipolar();
            }
        }
    }

    fn scene(&self) -> (Bed, &'static str, f32, Option<(&'static str, f32)>) {
        let tick = self.tick.floor() as u64;
        let frac = (self.tick - tick as f64) as f32;
        let slot = self.carousel.at(tick);
        let cur: &Entry = slot.current;
        let t = (tick as f32 + frac) * cur.speed;
        let mut b = bed(cur.name, t);
        let mut prev = None;
        if let Some((from, progress)) = slot.incoming_from {
            let tp = (tick as f32 + frac) * from.speed;
            b = mix_bed(&bed(from.name, tp), &b, progress);
            prev = Some((from.name, tp));
        }
        (b, cur.name, t, prev)
    }

    fn advance_block(&mut self, frames: usize) -> Bed {
        let slot_index = self.tick.floor() as u64 / crate::carousel::SLOT;
        if let Some(last) = self.last_slot {
            if last != slot_index {
                self.fire(Shot::Heat(0.85));
                self.fire(Shot::Sweep {
                    from: 6000.0,
                    to: 180.0,
                    ms: 520.0,
                    level: 0.35,
                });
                self.fire(Shot::Kick {
                    hz: 52.0,
                    ms: 240.0,
                    level: 0.6,
                });
                self.prev_t = None;
            }
        }
        self.last_slot = Some(slot_index);

        let (b, name, t, _) = self.scene();
        if let Some((pname, pt)) = self.prev_t {
            if pname == name && beat(name, pt) != beat(name, t) {
                match name {
                    "rings" => self.fire(Shot::Ping),
                    "tunnel" => self.fire(Shot::Whoomp),
                    "curtain" => self.fire(Shot::Crackle),
                    "hero" => self.fire(Shot::Kick {
                        hz: 60.0,
                        ms: 200.0,
                        level: 0.7,
                    }),
                    _ => {}
                }
            }
        }
        self.prev_t = Some((name, t));

        let step = frames as f64 / self.tick_len;
        self.tick = (self.tick + step).min(self.anchor + AHEAD);
        b
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let frames = out.len() / 2;
        let b = self.advance_block(frames);
        let rate = self.rate;
        let k_slow = coef(6.0, rate);
        let k_fast = coef(40.0, rate);
        let scan_target = if self.scan_on { 1.0 } else { 0.0 };

        for frame in out.as_chunks_mut::<2>().0 {
            if self.fade_wait > 0 {
                self.fade_wait -= 1;
                if self.fade_wait == 0 {
                    self.fading = true;
                }
            }
            let fade_target = if self.fading { 0.0 } else { 1.0 };
            let mut i = 0;
            while i < self.queue.len() {
                if self.queue[i].0 == 0 {
                    let (_, shot) = self.queue.swap_remove(i);
                    self.trigger(shot);
                } else {
                    self.queue[i].0 -= 1;
                    i += 1;
                }
            }

            let scan = self.scan.run(scan_target, k_slow, k_slow);
            let stinger = self.stinger.run(self.stinger_k);
            let heat = self.glitch.heat.run(b.heat.max(stinger), k_fast, k_slow);

            let cut = self
                .drone
                .cut
                .run(b.drone_cut * (1.0 + 1.5 * scan), k_slow, k_slow);
            let lvl = self.drone.level.run(
                b.drone_level * (1.0 - 0.6 * self.chord.env.value),
                k_slow,
                k_slow,
            );
            let pa = self.drone.a.step(b.drone_hz * 1.004, rate);
            let pb = self.drone.b.step(b.drone_hz * 0.996, rate);
            let raw = lerp(saw(pa) + saw(pb), square(pa) + square(pb), b.drone_shape) * 0.5;
            let filtered = self.drone.filter.run(raw, cut, 1.3, rate).low;
            let driven = lerp(filtered, hard(filtered, 4.0) * 0.6, b.drone_grit);
            let sub = sine(self.drone.sub.step(b.drone_hz * 0.5, rate)) * b.sub_level;
            let drone = driven * lvl + sub * lvl;

            let spike = self.ring.spike.run(coef(2.0, rate));
            let vib = sine(self.ring.vib.step(5.3, rate)) * 0.004;
            let trem = 0.75 + 0.25 * sine(self.ring.trem.step(0.7, rate));
            let ring_hz = b.ring_hz * (1.0 + vib + 0.3 * heat + 0.2 * spike);
            let ring_lvl = self.ring.level.run(
                b.ring_level + 0.5 * heat + 0.4 * spike + 0.08 * scan,
                k_fast,
                k_slow,
            );
            let ring = sine(self.ring.osc.step(ring_hz, rate)) * trem * ring_lvl;

            let wcut = self.wind.cut.run(b.wind_cut, k_slow, k_slow);
            let wlvl = self.wind.level.run(b.wind_level, k_slow, k_slow);
            let wind = self
                .wind
                .filter
                .run(self.rng.bipolar(), wcut, b.wind_q, rate)
                .band
                * wlvl;

            let plvl = self.pulse.level.run(b.pulse_level, k_slow, k_slow);
            let mut pulse = 0.0;
            if b.pulse_bpm > 0.0 {
                let p = self.pulse.clock.step(b.pulse_bpm / 60.0, rate);
                let crossed = |a: f32, b: f32, m: f32| (a < m && b >= m) || (b < a && m <= b);
                if crossed(self.pulse.was, p, 0.0) || crossed(self.pulse.was, p, 0.20) {
                    self.pulse.env.fire();
                    self.pulse.beat.phase = 0.0;
                }
                self.pulse.was = p;
                let env = self.pulse.env.run(coef(9.0, rate));
                pulse = sine(self.pulse.beat.step(42.0 + 70.0 * env, rate)) * env * plvl;
            }

            let crackle_burst = self.crackle_burst.run(coef(12.0, rate));
            let chance = |rate_hz: f32| rate_hz / rate;
            if self.rng.unit() < chance(b.chatter_rate + 80.0 * scan) {
                let hz = if self.scan_on {
                    2200.0
                } else {
                    600.0 + 1800.0 * self.rng.unit()
                };
                self.fire(Shot::Blip {
                    hz,
                    ms: 14.0,
                    crush: 0.8,
                });
            }
            let mut crackle = 0.0;
            if self.rng.unit() < chance(b.crackle_rate + 900.0 * crackle_burst) {
                crackle = self.rng.bipolar() * b.crackle_level.max(0.3 * crackle_burst);
            }

            let _ = self.scan_clock.step(14.0, rate);

            let mut bed_mix = drone + wind + pulse + crackle;
            let mut hi = ring;

            let mut shots = 0.0;
            for blip in self.blips.iter_mut() {
                if !blip.env.live() {
                    continue;
                }
                let env = blip.env.run(blip.k);
                let x = square(blip.osc.step(blip.hz, rate)) * env * 0.5;
                let hold = 1.0 + 9.0 * blip.crush;
                let bits = 16.0 - 11.0 * blip.crush;
                shots +=
                    lerp(x, blip.crusher.run(x, hold, bits), blip.crush) * b.chatter_level.max(0.5);
            }
            if self.ping.env.live() {
                let env = self.ping.env.run(self.ping.k);
                self.ping.hz = lerp(self.ping.hz, 880.0, coef(6.0, rate));
                hi += sine(self.ping.osc.step(self.ping.hz, rate)) * env * 0.35;
            }
            if self.kick.env.live() {
                let env = self.kick.env.run(self.kick.k);
                bed_mix += sine(self.kick.osc.step(self.kick.hz * (0.6 + env), rate))
                    * env
                    * self.kick.level;
            }
            if self.sweep.env.live() {
                let env = self.sweep.env.run(self.sweep.k);
                let hz = self.sweep.to + (self.sweep.from - self.sweep.to) * env;
                let noise = self
                    .sweep
                    .filter
                    .run(self.rng.bipolar(), hz, 5.0, rate)
                    .band;
                let tone = sine(self.sweep.osc.step(hz, rate));
                shots += (noise * 1.6 + tone * 0.35) * env.sqrt() * self.sweep.level;
            }
            if self.growl.env.live() {
                let env = self.growl.env.run(self.growl.k);
                let c = saw(self.growl.carrier.step(110.0 - 30.0 * (1.0 - env), rate));
                let m = sine(self.growl.mod_.step(43.0, rate));
                shots += hard(c * m, 6.0) * env * 0.55;
            }
            if self.chord.gate > 0.0 {
                self.chord.age += 1;
                if self.chord.age as f32 > rate * 0.9 {
                    self.chord.gate = 0.0;
                }
            }
            let cenv = self
                .chord
                .env
                .run(self.chord.gate, coef(3.0, rate), coef(1.2, rate));
            if cenv > 0.0005 {
                let mut c = 0.0;
                for (osc, hz) in self.chord.osc.iter_mut().zip([220.0, 330.0, 440.0]) {
                    c += sine(osc.step(hz, rate));
                }
                hi += soft(c) * cenv * 0.35;
            }

            let dry = (bed_mix + shots) * BED_GAIN;

            let mut wet = dry;
            if heat > HEAT_ON {
                if self.glitch.recut == 0 {
                    let slice =
                        (rate * (0.012 + 0.05 * self.rng.unit()) * (1.0 - 0.5 * heat)) as usize;
                    self.glitch.stutter.cut(slice);
                    self.glitch.recut = (slice as f32 * (2.0 + 3.0 * self.rng.unit())) as u32;
                } else {
                    self.glitch.recut -= 1;
                }
                wet = self.glitch.stutter.run(wet);
                wet = self
                    .glitch
                    .crusher
                    .run(wet, 1.0 + heat * heat * 14.0, 16.0 - heat * 11.0);
                let scream_hz = 400.0 + 5000.0 * heat * heat;
                let scream = self
                    .glitch
                    .scream
                    .run(self.rng.bipolar(), scream_hz, 6.0, rate)
                    .band;
                wet = hard(wet, 1.0 + 3.0 * heat) + scream * heat * 0.6;
            } else if self.glitch.stutter.engaged() {
                self.glitch.stutter.release();
                self.glitch.recut = 0;
            }
            let mono = lerp(dry, wet, (heat * 2.0).min(1.0));

            let side = 0.18 * ring - 0.12 * crackle + self.glitch.pan * 0.25 * (mono - dry);
            let fade = self.fade.run(fade_target, coef(2.0, rate), coef(3.0, rate));
            let gain = self.volume * fade;
            frame[0] = soft((mono + hi + side) * 1.3) * gain;
            frame[1] = soft((mono + hi - side) * 1.3) * gain;
        }
    }
}

pub fn energy(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carousel::SLOT;
    use crate::effects::ENTRIES;

    const RATE: f32 = 48000.0;

    fn engine() -> Engine {
        Engine::new(RATE, 120, Carousel::new(7), 1.0)
    }

    fn run(e: &mut Engine, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        for chunk in out.chunks_mut(1024) {
            e.render(chunk);
        }
        out
    }

    #[test]
    fn every_effect_has_a_bed_and_sounds() {
        for entry in ENTRIES {
            let b = bed(entry.name, 100.0);
            assert!(b.drone_level > 0.0, "{} has no drone", entry.name);
            let mut e = engine();
            let slot = (0..ENTRIES.len() as u64)
                .find(|s| Carousel::new(7).at(s * SLOT).current.name == entry.name)
                .unwrap();
            e.sync(slot * SLOT + 20);
            run(&mut e, 24000);
            let out = run(&mut e, 24000);
            assert!(energy(&out) > 0.01, "{} is silent", entry.name);
            assert!(out.iter().all(|x| x.abs() <= 1.0), "{} clips", entry.name);
        }
    }

    #[test]
    fn a_fail_cue_is_louder_than_the_bed() {
        let mut e = engine();
        e.sync(20);
        run(&mut e, 48000);
        let quiet = energy(&run(&mut e, 4800));
        e.cue(Cue::Fail);
        let loud = energy(&run(&mut e, 4800));
        assert!(loud > quiet * 1.5, "fail {loud} vs bed {quiet}");
    }

    #[test]
    fn a_key_blip_dies_out() {
        let mut e = Engine::new(RATE, 120, Carousel::new(7), 1.0);
        e.sync(20);
        run(&mut e, 48000);
        let base = energy(&run(&mut e, 4800));
        e.cue(Cue::Key);
        let hit = energy(&run(&mut e, 960));
        run(&mut e, 24000);
        let after = energy(&run(&mut e, 4800));
        assert!(hit > base, "the key made no sound");
        assert!(after < hit, "the blip never decayed");
    }

    #[test]
    fn the_fade_reaches_silence() {
        let mut e = engine();
        e.sync(5);
        run(&mut e, 24000);
        e.cue(Cue::Fade);
        run(&mut e, 96000);
        assert!(e.finished());
        assert!(energy(&run(&mut e, 4800)) < 1e-3);
    }

    #[test]
    fn the_clock_never_runs_far_ahead_of_the_picture() {
        let mut e = engine();
        e.sync(10);
        run(&mut e, 48000 * 4);
        assert!(e.tick <= 10.0 + AHEAD + 1e-6, "tick ran to {}", e.tick);
        e.sync(40);
        assert_eq!(e.tick, 40.0);
    }

    #[test]
    fn a_slot_change_fires_a_stinger() {
        let mut e = engine();
        e.sync(SLOT - 2);
        run(&mut e, 4800);
        assert!(!e.stinger.live());
        e.sync(SLOT);
        run(&mut e, 1024);
        assert!(e.stinger.live(), "no stinger on the slot boundary");
    }

    #[test]
    fn beats_step_with_the_picture() {
        for entry in ENTRIES {
            let stepped = (0..SLOT * 4).any(|t| {
                beat(entry.name, t as f32 * entry.speed)
                    != beat(entry.name, (t + 1) as f32 * entry.speed)
            });
            let expected = matches!(entry.name, "tunnel" | "rings" | "curtain" | "hero");
            assert_eq!(stepped, expected, "{} beat behaviour", entry.name);
        }
    }
}
