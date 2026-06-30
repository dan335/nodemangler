//! YCbCr (BT.709) color space conversions.
//!
//! Digital YCbCr using the Rec. 709 (HD) luma coefficients, full range. This is
//! the encoding used by H.264/H.265 HD video, and differs from the analog
//! BT.601 [`Y'UV`](super::yuv) space already provided. Y is luma in `0..1`; Cb
//! (blue-difference) and Cr (red-difference) are in `-0.5..0.5`, centered on 0
//! for neutral colors.

use crate::color::Color;

// Rec. 709 luma coefficients.
const KR: f32 = 0.2126;
const KG: f32 = 0.7152;
const KB: f32 = 0.0722;

impl Color {
    /// Creates an sRGB [`Color`] from full-range BT.709 YCbCr components.
    ///
    /// * `y` -- luma, `0.0..=1.0`
    /// * `cb` -- blue-difference chroma, `-0.5..=0.5`
    /// * `cr` -- red-difference chroma, `-0.5..=0.5`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_ycbcr(y: f32, cb: f32, cr: f32, alpha: f32) -> Color {
        let r = y + 2.0 * (1.0 - KR) * cr;
        let b = y + 2.0 * (1.0 - KB) * cb;
        // Recover green from the luma definition Y = KR*R + KG*G + KB*B.
        let g = (y - KR * r - KB * b) / KG;
        Color { r, g, b, a: alpha }
    }

    /// Converts this sRGB color to full-range BT.709 YCbCr components.
    ///
    /// Returns `(Y, Cb, Cr, alpha)`.
    pub fn to_ycbcr(&self) -> (f32, f32, f32, f32) {
        let y = KR * self.r + KG * self.g + KB * self.b;
        let cb = (self.b - y) / (2.0 * (1.0 - KB));
        let cr = (self.r - y) / (2.0 * (1.0 - KR));
        (y, cb, cr, self.a)
    }
}

#[cfg(test)]
#[path = "ycbcr_tests.rs"]
mod tests;
