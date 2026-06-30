//! Oklch color space conversions.
//!
//! Oklch is the cylindrical (polar) form of [Oklab](super::oklab): perceptual
//! lightness, chroma, and hue. Hue is expressed in degrees. It is to Oklab what
//! LCH is to Lab, and is the recommended space for perceptual gradients.

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from Oklch components.
    ///
    /// * `l` -- perceptual lightness, `0.0..=1.0`
    /// * `c` -- chroma, `0.0..≈0.4`
    /// * `h` -- hue in degrees, `0..360`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_oklch(l: f32, c: f32, h: f32, alpha: f32) -> Color {
        let a = c * h.to_radians().cos();
        let b = c * h.to_radians().sin();
        Color::from_oklab(l, a, b, alpha)
    }

    /// Converts this sRGB color to Oklch components.
    ///
    /// Returns `(L, C, hue_degrees, alpha)` with L in `0.0..=1.0`, C in
    /// `0.0..≈0.4`, and hue in `0..360`.
    pub fn to_oklch(&self) -> (f32, f32, f32, f32) {
        let (l, a, b, alpha) = self.to_oklab();
        let c = (a * a + b * b).sqrt();
        let mut h = b.atan2(a).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        (l, c, h, alpha)
    }
}

#[cfg(test)]
#[path = "oklch_tests.rs"]
mod tests;
