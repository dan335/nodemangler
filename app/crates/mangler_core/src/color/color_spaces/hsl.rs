//! HSL (Hue, Saturation, Lightness) color space conversions.
//!
//! Implements bidirectional conversion between sRGB and HSL using the algorithm
//! described at <https://en.wikipedia.org/wiki/HSL_and_HSV>.

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from HSL components.
    ///
    /// * `hue` -- degrees in `0..360`
    /// * `saturation` -- `0.0..=1.0`
    /// * `lightness` -- `0.0..=1.0`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_hsl(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Color {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let hue_prime = hue / 60.0;
        let largest_component = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());
        let (r_temp, g_temp, b_temp) = if hue_prime < 1.0 {
            (chroma, largest_component, 0.0)
        } else if hue_prime < 2.0 {
            (largest_component, chroma, 0.0)
        } else if hue_prime < 3.0 {
            (0.0, chroma, largest_component)
        } else if hue_prime < 4.0 {
            (0.0, largest_component, chroma)
        } else if hue_prime < 5.0 {
            (largest_component, 0.0, chroma)
        } else {
            (chroma, 0.0, largest_component)
        };
        // Shift each channel by the lightness offset to produce final RGB
        let lightness_match = lightness - chroma / 2.0;
        Color {
            r: r_temp + lightness_match,
            g: g_temp + lightness_match,
            b: b_temp + lightness_match,
            a: alpha,
        }
    }

    /// Converts this sRGB color to HSL components.
    ///
    /// Returns `(hue, saturation, lightness, alpha)` where hue is in degrees
    /// `0..360`, and saturation/lightness are in `0.0..=1.0`.
    pub fn to_hsl(&self) -> (f32, f32, f32, f32) {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
        let x_max = self.r.max(self.g.max(self.b));
        let x_min = self.r.min(self.g.min(self.b));
        let chroma = x_max - x_min;
        let lightness = (x_max + x_min) / 2.0;
        let hue = if chroma == 0.0 {
            0.0
        } else if self.r == x_max {
            60.0 * (self.g - self.b) / chroma
        } else if self.g == x_max {
            60.0 * (2.0 + (self.b - self.r) / chroma)
        } else {
            60.0 * (4.0 + (self.r - self.g) / chroma)
        };
        // Wrap negative hue into the 0..360 range
        let hue = if hue < 0.0 { 360.0 + hue } else { hue };
        let saturation = if lightness <= 0.0 || lightness >= 1.0 {
            0.0
        } else {
            (x_max - lightness) / lightness.min(1.0 - lightness)
        };

        (hue, saturation, lightness, self.a)
    }
}


#[cfg(test)]
#[path = "hsl_tests.rs"]
mod tests;
