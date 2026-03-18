//! HSV (Hue, Saturation, Value) color space conversions.
//!
//! Implements bidirectional conversion between sRGB and HSV using the algorithm
//! described at <https://en.wikipedia.org/wiki/HSL_and_HSV>.

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from HSV components.
    ///
    /// * `hue` -- degrees in `0..360`
    /// * `saturation` -- `0.0..=1.0`
    /// * `value` -- `0.0..=1.0`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_hsv(hue: f32, saturation: f32, value: f32, alpha: f32) -> Color {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_RGB
        let chroma = value * saturation;
        let hue_prime = hue / 60.0;
        let x = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());
        
        let (r_temp, g_temp, b_temp) = if hue_prime < 1.0 {
            (chroma, x, 0.0)
        } else if hue_prime < 2.0 {
            (x, chroma, 0.0)
        } else if hue_prime < 3.0 {
            (0.0, chroma, x)
        } else if hue_prime < 4.0 {
            (0.0, x, chroma)
        } else if hue_prime < 5.0 {
            (x, 0.0, chroma)
        } else {
            (chroma, 0.0, x)
        };
        
        // Shift each channel by the value offset to produce final RGB
        let m = value - chroma;
        
        Color {
            r: r_temp + m,
            g: g_temp + m,
            b: b_temp + m,
            a: alpha,
        }
    }

    /// Converts this sRGB color to HSV components.
    ///
    /// Returns `(hue, saturation, value, alpha)` where hue is in degrees
    /// `0..360`, and saturation/value are in `0.0..=1.0`.
    pub fn to_hsv(&self) -> (f32, f32, f32, f32) {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
        let x_max = self.r.max(self.g.max(self.b));
        let x_min = self.r.min(self.g.min(self.b));
        let chroma = x_max - x_min;
        
        let hue = if chroma == 0.0 {
            0.0
        } else if self.r == x_max {
            60.0 * (0.0 + (self.g - self.b) / chroma)
        } else if self.g == x_max {
            60.0 * (2.0 + (self.b - self.r) / chroma)
        } else {
            60.0 * (4.0 + (self.r - self.g) / chroma)
        };
        // Wrap negative hue into the 0..360 range
        let hue = if hue < 0.0 { 360.0 + hue } else { hue };
        
        let value = x_max;
        
        let saturation = if value == 0.0 {
            0.0
        } else {
            chroma / value
        };

        (hue, saturation, value, self.a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
        assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
        assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
        assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
        assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
    }

    #[test]
    fn test_hsv_roundtrip() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (h, s, v, a) = color.to_hsv();
        let color2 = Color::from_hsv(h, s, v, a);
        assert_color_approx(&color, &color2, EPSILON);
    }

    #[test]
    fn test_hsv_red() {
        let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let (h, s, v, _a) = color.to_hsv();
        assert!((h).abs() < EPSILON, "Hue: {}", h);
        assert!((s - 1.0).abs() < EPSILON, "Saturation: {}", s);
        assert!((v - 1.0).abs() < EPSILON, "Value: {}", v);
    }

    #[test]
    fn test_hsv_black() {
        let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let (_h, _s, v, _a) = color.to_hsv();
        assert!((v).abs() < EPSILON);
    }

    #[test]
    fn test_hsv_white() {
        let color = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let (_h, s, v, _a) = color.to_hsv();
        assert!((s).abs() < EPSILON);
        assert!((v - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_hsv_roundtrip_multiple() {
        let colors = [
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0),
            (0.5, 0.5, 0.5, 0.5),
            (0.2, 0.8, 0.4, 1.0),
        ];
        for (r, g, b, a) in colors {
            let color = Color::from_srgb_float(r, g, b, a);
            let hsv = color.to_hsv();
            let back = Color::from_hsv(hsv.0, hsv.1, hsv.2, hsv.3);
            assert_color_approx(&color, &back, EPSILON);
        }
    }
}