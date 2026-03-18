//! CMYK (Cyan, Magenta, Yellow, Key/Black) color space conversions.
//!
//! Implements a simple device-independent conversion between sRGB and CMYK
//! using the standard formulaic approach (no ICC profiles).

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from CMYK components.
    ///
    /// All values are in `0.0..=1.0`. The formula is `R = (1-C)(1-K)`, etc.
    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32, a: f32) -> Color {
        let r = (1.0 - c) * (1.0 - k);
        let g = (1.0 - m) * (1.0 - k);
        let b = (1.0 - y) * (1.0 - k);

        Color { r, g, b, a }
    }

    /// Converts this sRGB color to CMYK components.
    ///
    /// Returns `(c, m, y, k, alpha)` with all values in `0.0..=1.0`.
    /// A small epsilon (1e-10) is added to the denominator to avoid division by zero
    /// when the color is pure black (K=1).
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

    fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
        assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
        assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
        assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
        assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
    }

    #[test]
    fn test_cmyk_roundtrip() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (c, m, y, k, a) = color.to_cmyk();
        let color2 = Color::from_cmyk(c, m, y, k, a);
        assert_color_approx(&color, &color2, EPSILON);
    }

    #[test]
    fn test_cmyk_black() {
        let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let (_, _, _, k, _) = color.to_cmyk();
        assert!((k - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_cmyk_white() {
        let color = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let (c, m, y, k, _) = color.to_cmyk();
        assert!((c).abs() < EPSILON);
        assert!((m).abs() < EPSILON);
        assert!((y).abs() < EPSILON);
        assert!((k).abs() < EPSILON);
    }

    #[test]
    fn test_cmyk_red() {
        let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let (c, m, y, k, _) = color.to_cmyk();
        assert!((c).abs() < EPSILON);
        assert!((m - 1.0).abs() < EPSILON);
        assert!((y - 1.0).abs() < EPSILON);
        assert!((k).abs() < EPSILON);
    }

    #[test]
    fn test_cmyk_roundtrip_multiple() {
        let colors = [
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0),
            (0.5, 0.5, 0.5, 0.5),
            (0.2, 0.8, 0.4, 1.0),
        ];
        for (r, g, b, a) in colors {
            let color = Color::from_srgb_float(r, g, b, a);
            let cmyk = color.to_cmyk();
            let back = Color::from_cmyk(cmyk.0, cmyk.1, cmyk.2, cmyk.3, cmyk.4);
            assert_color_approx(&color, &back, EPSILON);
        }
    }
}