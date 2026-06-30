//! HWB (Hue, Whiteness, Blackness) color space conversions.
//!
//! HWB is an intuitive cylindrical model (CSS Color 4) closely related to HSV:
//! a pure hue mixed with white and black. Reference:
//! <https://en.wikipedia.org/wiki/HWB_color_model>

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from HWB components.
    ///
    /// * `hue` -- degrees in `0..360`
    /// * `whiteness` -- `0.0..=1.0`
    /// * `blackness` -- `0.0..=1.0`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_hwb(hue: f32, whiteness: f32, blackness: f32, alpha: f32) -> Color {
        // When whiteness + blackness >= 1 the hue washes out to an achromatic gray.
        if whiteness + blackness >= 1.0 {
            let gray = whiteness / (whiteness + blackness);
            return Color { r: gray, g: gray, b: gray, a: alpha };
        }
        // Take the pure hue (HSV with full saturation and value), then compress it
        // into the [whiteness, 1 - blackness] range.
        let pure = Color::from_hsv(hue, 1.0, 1.0, alpha);
        let scale = 1.0 - whiteness - blackness;
        Color {
            r: pure.r * scale + whiteness,
            g: pure.g * scale + whiteness,
            b: pure.b * scale + whiteness,
            a: alpha,
        }
    }

    /// Converts this sRGB color to HWB components.
    ///
    /// Returns `(hue, whiteness, blackness, alpha)` where hue is in degrees
    /// `0..360` and whiteness/blackness are in `0.0..=1.0`.
    pub fn to_hwb(&self) -> (f32, f32, f32, f32) {
        let hue = self.to_hsv().0;
        let whiteness = self.r.min(self.g).min(self.b);
        let blackness = 1.0 - self.r.max(self.g).max(self.b);
        (hue, whiteness, blackness, self.a)
    }
}

#[cfg(test)]
#[path = "hwb_tests.rs"]
mod tests;
