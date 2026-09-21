use crate::bg::dots::Dots;
use crate::bg::noise::bayer;

pub struct Field {
    pub w: usize,
    pub h: usize,
    pub row_pitch: f32,
    shrink: usize,
    pub buf: Vec<f32>,
}

impl Field {
    pub fn dots(d: &Dots) -> Self {
        Field::dots_shrunk(d, 1)
    }

    pub fn dots_shrunk(d: &Dots, shrink: usize) -> Self {
        let shrink = shrink.max(1);
        let (w, h) = (d.dw.div_ceil(shrink), d.dh.div_ceil(shrink));
        Field {
            w,
            h,
            row_pitch: 1.0,
            shrink,
            buf: vec![0.0; w * h],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, v: f32) {
        if x < self.w && y < self.h {
            self.buf[y * self.w + x] = v;
        }
    }

    /// Thresholds at full dot resolution whatever `shrink` is, so the Bayer cell
    /// still varies per dot and coarse sampling costs silhouette, not shading.
    pub fn dither_dots(&self, d: &mut Dots, bias: f32) {
        for y in 0..d.dh {
            let row = (y / self.shrink) * self.w;
            for x in 0..d.dw {
                if self.buf[row + x / self.shrink] > bayer(x, y) + bias {
                    d.set(x, y);
                }
            }
        }
    }
}
