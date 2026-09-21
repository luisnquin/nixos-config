use crate::bg::dots::Dots;
use crate::bg::noise::bayer;

pub struct Field {
    pub w: usize,
    pub h: usize,
    pub row_pitch: f32,
    pub buf: Vec<f32>,
}

impl Field {
    pub fn dots(d: &Dots) -> Self {
        Field {
            w: d.dw,
            h: d.dh,
            row_pitch: 1.0,
            buf: vec![0.0; d.dw * d.dh],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, v: f32) {
        if x < self.w && y < self.h {
            self.buf[y * self.w + x] = v;
        }
    }

    pub fn dither_dots(&self, d: &mut Dots, bias: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                if self.buf[y * self.w + x] > bayer(x, y) + bias {
                    d.set(x, y);
                }
            }
        }
    }
}
