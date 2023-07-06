use crate::color::Color;

impl Color {
    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32, a: f32) -> Color {
        let r = (1.0 - c) * (1.0 - k);
        let g = (1.0 - m) * (1.0 - k);
        let b = (1.0 - y) * (1.0 - k);

        Color { r, g, b, a }
    }

    pub fn to_cmyk(&self) -> (f32, f32, f32, f32, f32) {
        let k = 1.0 - self.r.max(self.g).max(self.b);
        let c = (1.0 - self.r - k) / (1.0 - k + 1e-10);
        let m = (1.0 - self.g - k) / (1.0 - k + 1e-10);
        let y = (1.0 - self.b - k) / (1.0 - k + 1e-10);

        (c, m, y, k, self.a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-7;

    #[test]
    fn text_to_from_cmyk() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (c, m, y, k, a) = color.to_cmyk();
        let color2 = Color::from_cmyk(c, m, y, k, a);
        
        assert!(
            (color.r - color2.r).abs() < EPSILON,
            "Red channel mismatch: {} vs {}",
            color.r,
            color2.r
        );
        assert!(
            (color.g - color2.g).abs() < EPSILON,
            "Green channel mismatch: {} vs {}",
            color.g,
            color2.g
        );
        assert!(
            (color.b - color2.b).abs() < EPSILON,
            "Blue channel mismatch: {} vs {}",
            color.b,
            color2.b
        );
        assert!(
            (color.a - color2.a).abs() < EPSILON,
            "Alpha channel mismatch: {} vs {}",
            color.a,
            color2.a
        );
    }
}