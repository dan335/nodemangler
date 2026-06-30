//! CIE xyY color space conversions.
//!
//! xyY separates a color into its chromaticity (x, y) and luminance (Y). It is
//! derived from [`XYZ`](super::xyz) and is convenient for chromaticity-diagram
//! work and white-point analysis. For luminance-free (black) colors the
//! chromaticity is undefined and defaults to the D65 white point.

use crate::color::Color;

// D65 chromaticity, used when luminance is zero and chromaticity is undefined.
const D65_X: f32 = 0.3127;
const D65_Y: f32 = 0.3290;

impl Color {
    /// Creates an sRGB [`Color`] from CIE xyY components.
    ///
    /// * `x`, `y` -- chromaticity coordinates (each roughly `0..1`)
    /// * `big_y` -- luminance, `0.0..=1.0`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_xyy(x: f32, y: f32, big_y: f32, alpha: f32) -> Color {
        if y.abs() < 1e-10 {
            return Color::from_xyz(0.0, 0.0, 0.0, alpha);
        }
        let big_x = x * big_y / y;
        let big_z = (1.0 - x - y) * big_y / y;
        Color::from_xyz(big_x, big_y, big_z, alpha)
    }

    /// Converts this sRGB color to CIE xyY components.
    ///
    /// Returns `(x, y, Y, alpha)`. For black, chromaticity defaults to D65.
    pub fn to_xyy(&self) -> (f32, f32, f32, f32) {
        let (big_x, big_y, big_z, alpha) = self.to_xyz();
        let sum = big_x + big_y + big_z;
        if sum.abs() < 1e-10 {
            return (D65_X, D65_Y, 0.0, alpha);
        }
        (big_x / sum, big_y / sum, big_y, alpha)
    }
}

#[cfg(test)]
#[path = "xyy_tests.rs"]
mod tests;
