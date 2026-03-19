//! YUV color space conversions.
//!
//! Converts between sRGB and YUV using BT.601 luma/chroma coefficients.
//! Y is the luma (brightness) component; U and V are blue-difference and
//! red-difference chroma components respectively.

use crate::color::Color;

/// BT.601 coefficients for RGB-to-YUV: [Wr, Wg, Wb, Umax, Vmax].
static RGB2YUV_COEFFS: [f32; 5] = [0.299, 0.587, 0.114, 0.492, 0.877];
/// BT.601 coefficients for YUV-to-RGB: [Ub, Ug, Vg, Vr].
static YUV2RGB_COEFFS: [f32; 4] = [2.032, -0.395, -0.581, 1.14];

impl Color {
    /// Creates an sRGB [`Color`] from YUV components.
    ///
    /// Applies the BT.601 inverse transform to recover RGB channels.
    pub fn from_yuv(y: f32, u: f32, v: f32, a: f32) -> Color {
        let r = y + YUV2RGB_COEFFS[3] * v;
        let g = y + YUV2RGB_COEFFS[1] * u + YUV2RGB_COEFFS[2] * v;
        let b = y + YUV2RGB_COEFFS[0] * u;
        Color { r, g, b, a }
    }

    /// Converts this sRGB color to YUV components.
    ///
    /// Returns `(Y, U, V, alpha)` using the BT.601 luma/chroma coefficients.
    pub fn to_yuv(&self) -> (f32, f32, f32, f32) {
        let y = RGB2YUV_COEFFS[0] * self.r + RGB2YUV_COEFFS[1] * self.g + RGB2YUV_COEFFS[2] * self.b;
        let u = RGB2YUV_COEFFS[3] * (self.b - y);
        let v = RGB2YUV_COEFFS[4] * (self.r - y);
        (y, u, v, self.a)
    }
}

#[cfg(test)]
#[path = "yuv_tests.rs"]
mod tests;
