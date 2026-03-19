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
#[path = "cmyk_tests.rs"]
mod tests;
