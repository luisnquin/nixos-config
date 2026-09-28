use image::{Rgba, RgbaImage};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub per_unit: f64,
    pub origin: (f64, f64),
}

const MOST_LINES: f64 = 12.0;
const STEPS: [u32; 7] = [25, 50, 100, 200, 250, 500, 1000];

// 3x5 glyphs, a row per entry, bit 4 the left column
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

const LINE: Rgba<u8> = Rgba([255, 0, 200, 255]);
const INK: Rgba<u8> = Rgba([255, 235, 0, 255]);
const PAPER: Rgba<u8> = Rgba([0, 0, 0, 255]);

pub fn step(image: &RgbaImage, grid: Grid) -> u32 {
    let span = f64::from(image.width().max(image.height())) / grid.per_unit;

    STEPS
        .into_iter()
        .find(|s| span / f64::from(*s) <= MOST_LINES)
        .unwrap_or(STEPS[STEPS.len() - 1])
}

pub fn draw(image: &mut RgbaImage, grid: Grid) {
    let step = f64::from(step(image, grid));
    let dot = (image.width().min(image.height()) / 270).max(1);

    for vertical in [true, false] {
        let (origin, extent) = match vertical {
            true => (grid.origin.0, image.width()),
            false => (grid.origin.1, image.height()),
        };

        let mut at = ((origin / step).floor() + 1.0) * step;

        loop {
            let px = ((at - origin) * grid.per_unit).round() as u32;

            if px >= extent {
                break;
            }

            line(image, vertical, px, dot.div_ceil(2));

            let (x, y) = match vertical {
                true => (px + dot * 2, dot * 2),
                false => (dot * 2, px + dot * 2),
            };

            label(image, x, y, dot, at as u32);
            at += step;
        }
    }
}

fn line(image: &mut RgbaImage, vertical: bool, at: u32, width: u32) {
    let (w, h) = image.dimensions();

    for offset in 0..width {
        for along in 0..if vertical { h } else { w } {
            let (x, y) = if vertical {
                (at + offset, along)
            } else {
                (along, at + offset)
            };

            if x < w && y < h {
                image.put_pixel(x, y, LINE);
            }
        }
    }
}

fn label(image: &mut RgbaImage, x: u32, y: u32, dot: u32, value: u32) {
    let text = value.to_string();
    let glyph = 4 * dot;
    let width = glyph * text.len() as u32 + dot;

    fill(image, x, y, width, 7 * dot, PAPER);

    for (i, c) in text.bytes().enumerate() {
        let rows = DIGITS[usize::from(c - b'0')];
        let left = x + dot + glyph * i as u32;

        for (row, bits) in rows.iter().enumerate() {
            for col in 0..3 {
                if bits & (4 >> col) != 0 {
                    fill(
                        image,
                        left + col * dot,
                        y + dot + row as u32 * dot,
                        dot,
                        dot,
                        INK,
                    );
                }
            }
        }
    }
}

fn fill(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for py in y..(y + h).min(image.height()) {
        for px in x..(x + w).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]))
    }

    #[test]
    fn a_small_crop_gets_finer_lines_than_a_full_frame() {
        let grid = Grid {
            per_unit: 1.0,
            origin: (0.0, 0.0),
        };

        assert_eq!(step(&frame(1080, 2400), grid), 200);
        assert_eq!(step(&frame(400, 200), grid), 50);
    }

    #[test]
    fn lines_land_where_the_tap_unit_does() {
        let mut image = frame(1179, 2556);
        let grid = Grid {
            per_unit: 3.0,
            origin: (0.0, 0.0),
        };

        draw(&mut image, grid);

        assert_eq!(*image.get_pixel(300, 1000), LINE);
        assert_ne!(*image.get_pixel(299, 1000), LINE);
    }

    #[test]
    fn a_crop_is_labelled_from_where_it_starts() {
        let mut image = frame(300, 300);
        let grid = Grid {
            per_unit: 1.0,
            origin: (130.0, 0.0),
        };

        draw(&mut image, grid);

        assert_eq!(*image.get_pixel(20, 150), LINE);
    }
}
