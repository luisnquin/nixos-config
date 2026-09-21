use std::f32::consts::TAU;

pub struct Surge {
    pub period: f32,
}

impl Surge {
    fn w(&self) -> f32 {
        TAU / self.period.max(f32::EPSILON)
    }

    pub fn level(&self, t: f32) -> f32 {
        let s = 0.5 + 0.5 * (self.w() * t).sin();
        s * s
    }

    pub fn integral(&self, t: f32) -> f32 {
        let w = self.w();
        0.375 * t - (0.5 / w) * (w * t).cos() - (0.125 / (2.0 * w)) * (2.0 * w * t).sin()
    }

    /// Phase is the integral of the rate, never a product: under
    /// `base * t * (1 + surge * level(t))` a falling envelope runs time back.
    pub fn phase(&self, base: f32, surge: f32, t: f32) -> f32 {
        base * (t + surge * self.integral(t))
    }
}
