pub const RAMP_FULL: &[u8] = b" .:-=+*#%@";

pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub buf: Vec<u8>,
}

impl Grid {
    pub fn new(w: usize, h: usize) -> Self {
        Grid {
            w,
            h,
            buf: vec![b' '; w * h],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, c: u8) {
        if x < self.w && y < self.h {
            self.buf[y * self.w + x] = c;
        }
    }
}

pub fn ramp_pick(ramp: &[u8], v: f32) -> u8 {
    let i = (v.clamp(0.0, 1.0) * (ramp.len() - 1) as f32).round() as usize;
    ramp[i]
}
