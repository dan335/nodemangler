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
#[path = "rgb_linear_tests.rs"]
mod tests;
