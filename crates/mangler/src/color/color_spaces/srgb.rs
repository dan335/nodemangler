use crate::color::Color;

impl Color {
    // 0 - 255
    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r:r as f32 / u8::MAX as f32,
            g:g as f32 / u8::MAX as f32,
            b:b as f32 / u8::MAX as f32,
            a:a as f32 / u8::MAX as f32,
        }
    }

    // 0.0 - 1.0
    pub fn from_srgb_float(r:f32, g:f32, b:f32, a:f32) -> Color {
        Color {
            r,
            g,
            b,
            a,
        }
    }

    pub fn to_srgb_u8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    pub fn to_srgb_float(&self) -> (f32, f32, f32, f32) {
        (
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
            self.a.clamp(0.0, 1.0),
        )
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn text_to_from_srgb() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (r, g, b, a) = color.to_srgb_float();
        let color2 = Color::from_srgb_float(r, g, b, a);
        assert_eq!(color, color2);
    }
}