pub fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32)
        .wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x8564_5f9f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    h
}

pub fn hash01(x: i32, y: i32) -> f32 {
    hash2(x, y) as f32 / u32::MAX as f32
}

pub fn value_noise(x: f32, y: f32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = x - xi;
    let yf = y - yi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let (ix, iy) = (xi as i32, yi as i32);

    let a = hash01(ix, iy);
    let b = hash01(ix + 1, iy);
    let c = hash01(ix, iy + 1);
    let d = hash01(ix + 1, iy + 1);

    (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v
}

pub fn fbm(x: f32, y: f32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    let (mut px, mut py) = (x, y);
    for _ in 0..octaves {
        sum += amp * value_noise(px, py);
        norm += amp;
        amp *= 0.5;
        let nx = px * 2.03 + 11.7;
        let ny = py * 2.03 - 5.3;
        px = nx;
        py = ny;
    }
    if norm == 0.0 {
        0.0
    } else {
        sum / norm
    }
}

#[rustfmt::skip]
pub const BAYER8: [u8; 64] = [
     0, 32,  8, 40,  2, 34, 10, 42,
    48, 16, 56, 24, 50, 18, 58, 26,
    12, 44,  4, 36, 14, 46,  6, 38,
    60, 28, 52, 20, 62, 30, 54, 22,
     3, 35, 11, 43,  1, 33,  9, 41,
    51, 19, 59, 27, 49, 17, 57, 25,
    15, 47,  7, 39, 13, 45,  5, 37,
    63, 31, 55, 23, 61, 29, 53, 21,
];

pub fn bayer(x: usize, y: usize) -> f32 {
    (BAYER8[(y & 7) * 8 + (x & 7)] as f32 + 0.5) / 64.0
}
