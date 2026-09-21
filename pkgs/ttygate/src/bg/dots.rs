pub const CELL_W: usize = 2;
pub const CELL_H: usize = 4;

const BITS: [[u8; CELL_H]; CELL_W] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

pub struct Dots {
    pub dw: usize,
    pub dh: usize,
    pub buf: Vec<bool>,
}

impl Dots {
    pub fn new(w: usize, h: usize) -> Self {
        let (dw, dh) = (w * CELL_W, h * CELL_H);
        Dots {
            dw,
            dh,
            buf: vec![false; dw * dh],
        }
    }

    pub fn set(&mut self, x: usize, y: usize) {
        self.put(x, y, true);
    }

    pub fn put(&mut self, x: usize, y: usize, on: bool) {
        if x < self.dw && y < self.dh {
            self.buf[y * self.dw + x] = on;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        x < self.dw && y < self.dh && self.buf[y * self.dw + x]
    }

    pub fn mask(&self, cx: usize, cy: usize) -> u8 {
        let mut m = 0u8;
        for (col, bits) in BITS.iter().enumerate() {
            for (row, bit) in bits.iter().enumerate() {
                if self.get(cx * CELL_W + col, cy * CELL_H + row) {
                    m |= bit;
                }
            }
        }
        m
    }
}
