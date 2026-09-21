use crate::dots::Dots;
use crate::field::Field;

const CAM_Z: f32 = 3.0;
const FOV: f32 = 0.45;
const YAW: f32 = 0.20;
const YAW_RATE: f32 = 0.013;

const MAX_STEPS: u32 = 96;
const MAX_DIST: f32 = 6.5;
const SURF_EPS: f32 = 0.0015;
const BOUND: f32 = 1.25;
const STEP_SCALE: f32 = 0.85;

const AO_STEP: f32 = 0.045;
const AO_RATIO: f32 = 2.10;
const AO_TAPS: u32 = 5;
const AO_FALLOFF: f32 = 0.75;
const AO_GAIN: f32 = 0.6;
const AO_SPREAD: f32 = 0.75;
const AO_TURN: f32 = 2.399_963;

const SHADOW_STEPS: u32 = 28;
const SHADOW_NEAR: f32 = 0.02;
const SHADOW_FAR: f32 = 1.6;
const SHADOW_K: f32 = 12.0;
const SHADOW_MIN: f32 = 0.10;

#[derive(Clone, Copy)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn scale(self, k: f32) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
    fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn norm(self) -> V3 {
        let l = self.len();
        if l == 0.0 {
            self
        } else {
            self.scale(1.0 / l)
        }
    }
}

fn rot_y(p: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    v3(p.x * c + p.z * s, p.y, -p.x * s + p.z * c)
}

fn rot_z(p: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    v3(p.x * c - p.y * s, p.x * s + p.y * c, p.z)
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

fn smax(a: f32, b: f32, k: f32) -> f32 {
    -smin(-a, -b, k)
}

fn sd_ellipsoid(p: V3, r: V3) -> f32 {
    let q = v3(p.x / r.x, p.y / r.y, p.z / r.z);
    let k0 = q.len();
    if k0 == 0.0 {
        return -r.x.min(r.y).min(r.z);
    }
    let w = v3(q.x / r.x, q.y / r.y, q.z / r.z);
    let k1 = w.len();
    if k1 == 0.0 {
        return 0.0;
    }
    k0 * (k0 - 1.0) / k1
}

fn sd_round_box(p: V3, b: V3, r: f32) -> f32 {
    let q = v3(p.x.abs() - b.x, p.y.abs() - b.y, p.z.abs() - b.z);
    let outside = v3(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).len();
    let inside = q.x.max(q.y).max(q.z).min(0.0);
    outside + inside - r
}

fn sd_capsule(p: V3, a: V3, b: V3, r: f32) -> f32 {
    let pa = p.sub(a);
    let ba = b.sub(a);
    let denom = ba.dot(ba);
    let h = if denom == 0.0 {
        0.0
    } else {
        (pa.dot(ba) / denom).clamp(0.0, 1.0)
    };
    pa.sub(ba.scale(h)).len() - r
}

const ORBIT_AT: V3 = V3 {
    x: 0.265,
    y: 0.085,
    z: 0.24,
};
const ORBIT_HALF: V3 = V3 {
    x: 0.105,
    y: 0.140,
    z: 0.34,
};
const ORBIT_ROUND: f32 = 0.040;
const ORBIT_TILT: f32 = 0.15;

const TOOTH_PERIOD: f32 = 0.10;
const TOOTH_HALF: f32 = 0.017;

fn cranium(p: V3, q: V3) -> f32 {
    let vault = sd_ellipsoid(p.sub(v3(0.0, 0.40, -0.06)), v3(0.62, 0.58, 0.68));
    let face = sd_ellipsoid(p.sub(v3(0.0, -0.12, 0.16)), v3(0.45, 0.46, 0.50));
    let mut d = smin(vault, face, 0.06);

    let alveolar = sd_ellipsoid(p.sub(v3(0.0, -0.43, 0.17)), v3(0.34, 0.15, 0.37));
    d = smin(d, alveolar, 0.05);

    let brow = sd_ellipsoid(p.sub(v3(0.0, 0.28, 0.30)), v3(0.50, 0.075, 0.24));
    d = smin(d, brow, 0.04);

    let bridge = sd_ellipsoid(p.sub(v3(0.0, -0.02, 0.44)), v3(0.055, 0.17, 0.12));
    d = smin(d, bridge, 0.04);

    let fossa = sd_ellipsoid(q.sub(v3(1.40, 0.32, -0.06)), v3(0.90, 0.24, 2.20));
    smax(d, -fossa, 0.03)
}

fn zygoma(q: V3) -> f32 {
    let arch = smin(
        sd_capsule(q, v3(0.30, -0.08, 0.32), v3(0.62, -0.04, 0.02), 0.060),
        sd_capsule(q, v3(0.62, -0.04, 0.02), v3(0.50, 0.00, -0.32), 0.050),
        0.05,
    );
    let malar = sd_ellipsoid(q.sub(v3(0.34, -0.08, 0.28)), v3(0.13, 0.11, 0.13));
    let process = sd_capsule(q, v3(0.42, -0.06, 0.34), v3(0.39, 0.20, 0.28), 0.070);
    smin(smin(arch, malar, 0.06), process, 0.06)
}

fn mandible(p: V3, q: V3) -> f32 {
    let body = sd_ellipsoid(p.sub(v3(0.0, -0.68, 0.18)), v3(0.36, 0.21, 0.34));
    let chin = sd_ellipsoid(p.sub(v3(0.0, -0.80, 0.24)), v3(0.21, 0.13, 0.16));
    let mut d = smin(body, chin, 0.07);
    let ramus = sd_capsule(q, v3(0.41, -0.64, -0.02), v3(0.46, -0.08, -0.18), 0.075);
    d = smin(d, ramus, 0.08);
    d
}

fn orbit(q: V3) -> f32 {
    let r = rot_z(q.sub(ORBIT_AT), -ORBIT_TILT);
    sd_round_box(r, ORBIT_HALF, ORBIT_ROUND)
}

fn nasal(p: V3) -> f32 {
    let upper = sd_ellipsoid(p.sub(v3(0.0, -0.16, 0.30)), v3(0.050, 0.10, 0.42));
    let lower = sd_ellipsoid(p.sub(v3(0.0, -0.30, 0.30)), v3(0.150, 0.085, 0.42));
    smin(upper, lower, 0.07)
}

fn teeth(p: V3) -> f32 {
    let fx = p.x - (p.x / TOOTH_PERIOD).round() * TOOTH_PERIOD;
    let slot = sd_round_box(
        v3(fx, p.y + 0.515, p.z - 0.53),
        v3(TOOTH_HALF, 0.10, 0.13),
        0.008,
    );
    slot.max(p.x.abs() - 0.30)
}

pub fn map(p: V3) -> f32 {
    let q = v3(p.x.abs(), p.y, p.z);

    let mut d = cranium(p, q);
    d = smin(d, zygoma(q), 0.05);
    d = d.min(mandible(p, q));

    d = smax(d, -orbit(q), 0.015);
    d = smax(d, -nasal(p), 0.02);

    let gap = sd_round_box(p.sub(v3(0.0, -0.515, 0.45)), v3(0.34, 0.021, 0.30), 0.004);
    d = d.max(-gap);
    d = smax(d, -teeth(p), 0.006);

    d
}

fn basis(n: V3) -> (V3, V3) {
    let seed = if n.z.abs() < 0.9 {
        v3(0.0, 0.0, 1.0)
    } else {
        v3(1.0, 0.0, 0.0)
    };
    let t = seed.cross(n).norm();
    (t, n.cross(t))
}

fn occlusion(p: V3, n: V3) -> f32 {
    let (t, b) = basis(n);
    let mut occ = 0.0;
    let mut total = 0.0;
    let mut weight = 1.0;
    let mut h = AO_STEP;
    for i in 1..=AO_TAPS {
        let (s, c) = (i as f32 * AO_TURN).sin_cos();
        let r = AO_SPREAD * h;
        let q = p.add(n.scale(h)).add(t.scale(c * r)).add(b.scale(s * r));
        occ += weight * ((h - map(q)) / h).clamp(0.0, 1.0);
        total += weight;
        weight *= AO_FALLOFF;
        h *= AO_RATIO;
    }
    (1.0 - AO_GAIN * occ / total).clamp(0.0, 1.0)
}

fn shadow(p: V3, l: V3) -> f32 {
    let mut res: f32 = 1.0;
    let mut t = SHADOW_NEAR;
    for _ in 0..SHADOW_STEPS {
        let h = map(p.add(l.scale(t)));
        if h < SURF_EPS {
            return SHADOW_MIN;
        }
        res = res.min(SHADOW_K * h / t);
        t += h;
        if t > SHADOW_FAR {
            break;
        }
    }
    SHADOW_MIN + (1.0 - SHADOW_MIN) * res.clamp(0.0, 1.0)
}

fn normal(p: V3) -> V3 {
    const E: f32 = 0.002;
    v3(
        map(v3(p.x + E, p.y, p.z)) - map(v3(p.x - E, p.y, p.z)),
        map(v3(p.x, p.y + E, p.z)) - map(v3(p.x, p.y - E, p.z)),
        map(v3(p.x, p.y, p.z + E)) - map(v3(p.x, p.y, p.z - E)),
    )
    .norm()
}

#[derive(Clone, Copy)]
pub struct Place {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub yaw: f32,
}

pub const HALF_W: f32 = 0.62;
pub const HALF_H: f32 = 0.84;

const PAD: f32 = 3.0;

fn span_of(centre: f32, half: f32, n: usize) -> (usize, usize) {
    let n = n as i32;
    let lo = ((centre - half - PAD).floor() as i32).clamp(0, n);
    let hi = ((centre + half + PAD).ceil() as i32)
        .saturating_add(1)
        .clamp(lo, n);
    (lo as usize, hi as usize)
}

const SHRINK: usize = 2;

pub fn render_at(tick: f32, place: Place, d: &mut Dots) {
    let mut f = Field::dots_shrunk(d, SHRINK);
    shade(tick, place, &mut f);
    f.dither_dots(d, 0.0);
}

fn shade(tick: f32, place: Place, f: &mut Field) {
    if f.w == 0 || f.h == 0 {
        return;
    }
    let span = f.h as f32 * 0.5 * f.row_pitch;
    let cx = (f.w as f32 - 1.0) * 0.5 + place.dx * span;
    let cy = (f.h as f32 - 1.0) * 0.5 + place.dy * span / f.row_pitch;
    let unit = span * place.scale.max(f32::EPSILON);
    let angle = YAW * (tick * YAW_RATE).sin() + place.yaw;

    let ro = rot_y(v3(0.0, 0.0, CAM_Z), -angle);
    let light = rot_y(v3(0.12, 0.44, 0.89).norm(), -angle);

    let (x0, x1) = span_of(cx, HALF_W * place.scale * span, f.w);
    let (y0, y1) = span_of(cy, HALF_H * place.scale * span / f.row_pitch, f.h);

    for y in y0..y1 {
        let uy = (y as f32 - cy) * f.row_pitch / unit;
        for x in x0..x1 {
            let ux = (x as f32 - cx) / unit;
            let rd = rot_y(v3(ux * FOV, -uy * FOV, -1.0).norm(), -angle);

            let b = ro.dot(rd);
            let c = ro.dot(ro) - BOUND * BOUND;
            let disc = b * b - c;
            if disc < 0.0 {
                continue;
            }
            let entry = (-b - disc.sqrt()).max(0.0);

            let mut t = entry;
            let mut hit = false;
            for _ in 0..MAX_STEPS {
                let p = ro.add(rd.scale(t));
                let d = map(p);
                if d < SURF_EPS {
                    hit = true;
                    break;
                }
                t += d * STEP_SCALE;
                if t > MAX_DIST {
                    break;
                }
            }
            if !hit {
                continue;
            }

            let p = ro.add(rd.scale(t));
            let n = normal(p);
            let ao = occlusion(p, n);
            let sh = shadow(p.add(n.scale(SURF_EPS * 6.0)), light);
            let diff = 0.5 + 0.5 * n.dot(light);
            let rim = (1.0 - n.dot(rd.scale(-1.0)).max(0.0)).powi(4);
            let shade = (0.30 + 0.70 * diff * diff) * ao * sh + 0.30 * rim * ao;
            let v = ((shade - 0.30) * 1.9).clamp(0.0, 1.0);
            let v = v * v * (3.0 - 2.0 * v);
            f.set(x, y, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_box_covers_every_hit() {
        let (w, h) = (240usize, 67usize);
        let d = Dots::new(w, h);
        let f = Field::dots_shrunk(&d, SHRINK);
        let span = f.h as f32 * 0.5 * f.row_pitch;

        for i in 0..24 {
            let tick = i as f32 * 41.0;
            for &scale in &[0.55f32, 0.78, 1.0] {
                let place = Place { dx: 0.0, dy: 0.0, scale, yaw: 0.0 };
                let cx = (f.w as f32 - 1.0) * 0.5;
                let cy = (f.h as f32 - 1.0) * 0.5;
                let unit = span * scale;
                let angle = YAW * (tick * YAW_RATE).sin() + place.yaw;
                let ro = rot_y(v3(0.0, 0.0, CAM_Z), -angle);
                let (x0, x1) = span_of(cx, HALF_W * scale * span, f.w);
                let (y0, y1) = span_of(cy, HALF_H * scale * span / f.row_pitch, f.h);

                for y in 0..f.h {
                    let uy = (y as f32 - cy) * f.row_pitch / unit;
                    for x in 0..f.w {
                        if x >= x0 && x < x1 && y >= y0 && y < y1 {
                            continue;
                        }
                        let ux = (x as f32 - cx) / unit;
                        let rd = rot_y(v3(ux * FOV, -uy * FOV, -1.0).norm(), -angle);
                        let b = ro.dot(rd);
                        let disc = b * b - (ro.dot(ro) - BOUND * BOUND);
                        if disc < 0.0 {
                            continue;
                        }
                        let mut t = (-b - disc.sqrt()).max(0.0);
                        for _ in 0..MAX_STEPS {
                            let dist = map(ro.add(rd.scale(t)));
                            assert!(
                                dist >= SURF_EPS,
                                "hit outside the culled box at ({x}, {y}), tick {tick}, scale {scale}"
                            );
                            t += dist * STEP_SCALE;
                            if t > MAX_DIST {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
