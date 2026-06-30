//! CIE LCH (Lightness, Chroma, Hue) color space conversions.
//!
//! LCH is the cylindrical representation of CIE L*a*b*. To stay consistent with
//! the [`lab`](super::lab) module, these conversions delegate to
//! `to_lab`/`from_lab` (which use a D50 reference white) and only move between
//! the Cartesian (a, b) and polar (chroma, hue) forms. Lightness and chroma are
//! normalized by 100.

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from LCH components.
    ///
    /// * `lightness` -- normalized `0.0..=1.0` (internally scaled to L* `0..100`)
    /// * `chroma` -- normalized `0.0..=1.0` (internally scaled to `0..100`)
    /// * `hue` -- degrees `0..360`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_lch(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Color {
        let l = lightness * 100.0;
        let c = chroma * 100.0;
        let a = c * hue.to_radians().cos();
        let b = c * hue.to_radians().sin();
        Color::from_lab(l, a, b, alpha)
    }

    /// Converts this sRGB color to LCH components.
    ///
    /// Returns `(lightness, chroma, hue, alpha)` -- the cylindrical form of the
    /// (D50) CIE L*a*b* values. Lightness and chroma are normalized by 100 and
    /// clamped to `0.0..=1.5`; hue is in degrees `0..360`.
    pub fn to_lch(&self) -> (f32, f32, f32, f32) {
        let (l, a, b, alpha) = self.to_lab();
        let c = (a * a + b * b).sqrt();
        let mut h = b.atan2(a).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        ((l / 100.0).clamp(0.0, 1.5), (c / 100.0).clamp(0.0, 1.5), h, alpha)
    }
}

#[cfg(test)]
#[path = "lch_tests.rs"]
mod tests;
