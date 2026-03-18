//! Linear RGB color space conversions.
//!
//! Converts between sRGB (gamma-encoded) and linear (scene-referred) RGB
//! using the standard sRGB transfer function (IEC 61966-2-1).

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from linear RGB channel values.
    ///
    /// Applies the sRGB gamma encoding curve to each color channel.
    /// Alpha is stored as-is (it is always linear).
    pub fn from_rgb_linear(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
        Color {
            r: linear_to_nonlinear_srgb(red),
            g: linear_to_nonlinear_srgb(green),
            b: linear_to_nonlinear_srgb(blue),
            a: alpha,
        }
    }

    /// Converts this sRGB color to linear RGB channel values.
    ///
    /// Removes the sRGB gamma curve from each color channel.
    /// Alpha is returned as-is (it is always linear).
    pub fn to_rgb_linear(&self) -> (f32, f32, f32, f32) {
        (
            nonlinear_to_linear_rgb(self.r),
            nonlinear_to_linear_rgb(self.g),
            nonlinear_to_linear_rgb(self.b),
            self.a,
        )
    }
}

/// Converts a single sRGB (nonlinear / gamma-encoded) value to linear RGB.
///
/// Uses the standard sRGB piecewise transfer function: a linear segment
/// below 0.04045, and a power curve above.
#[inline]
pub fn nonlinear_to_linear_rgb(n: f32) -> f32 {
    if n <= 0.0 {
        return n;
    }
    if n <= 0.04045 {
        n / 12.92 // linear falloff in dark values
    } else {
        ((n + 0.055) / 1.055).powf(2.4) // gamma curve in other area
    }
}

/// Converts a single linear RGB value to sRGB (nonlinear / gamma-encoded).
///
/// Inverse of [`nonlinear_to_linear_rgb`]: a linear segment below 0.0031308,
/// and a power curve above.
#[inline]
pub fn linear_to_nonlinear_srgb(n: f32) -> f32 {
    if n <= 0.0 {
        return n;
    }

    if n <= 0.0031308 {
        n * 12.92 // linear falloff in dark values
    } else {
        (1.055 * n.powf(1.0 / 2.4)) - 0.055 // gamma curve in other area
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
    fn test_rgb_linear_roundtrip() {
        let color = Color::from_rgb_linear(0.75, 0.5, 0.25, 1.0);
        let (r, g, b, a) = color.to_rgb_linear();
        let color2 = Color::from_rgb_linear(r, g, b, a);
        assert_color_approx(&color, &color2, EPSILON);
    }

    #[test]
    fn test_rgb_linear_black() {
        let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let (r, g, b, a) = color.to_rgb_linear();
        assert!((r).abs() < EPSILON);
        assert!((g).abs() < EPSILON);
        assert!((b).abs() < EPSILON);
        assert!((a - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_rgb_linear_white() {
        let color = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let (r, g, b, _a) = color.to_rgb_linear();
        assert!((r - 1.0).abs() < EPSILON);
        assert!((g - 1.0).abs() < EPSILON);
        assert!((b - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_nonlinear_to_linear_zero() {
        assert_eq!(nonlinear_to_linear_rgb(0.0), 0.0);
    }

    #[test]
    fn test_linear_to_nonlinear_zero() {
        assert_eq!(linear_to_nonlinear_srgb(0.0), 0.0);
    }

    #[test]
    fn test_gamma_roundtrip() {
        for &val in &[0.0, 0.01, 0.04, 0.04045, 0.05, 0.1, 0.5, 0.9, 1.0] {
            let linear = nonlinear_to_linear_rgb(val);
            let back = linear_to_nonlinear_srgb(linear);
            assert!((val - back).abs() < EPSILON, "Roundtrip failed for {}: got {}", val, back);
        }
    }

    #[test]
    fn test_srgb_roundtrip_multiple_colors() {
        let colors = [
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0),
            (0.5, 0.5, 0.5, 0.5),
        ];
        for (r, g, b, a) in colors {
            let color = Color::from_srgb_float(r, g, b, a);
            let linear = color.to_rgb_linear();
            let back = Color::from_rgb_linear(linear.0, linear.1, linear.2, linear.3);
            assert_color_approx(&color, &back, EPSILON);
        }
    }
}