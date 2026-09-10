//! Color space definitions and conversions.
//!
//! Each submodule implements `from_*` and `to_*` methods on [`super::Color`] for
//! converting between sRGB and a target color space. The [`ColorSpace`] enum
//! enumerates all supported color spaces.

use serde::{Deserialize, Serialize};

pub mod srgb;
pub mod hsl;
pub mod hsv;
pub mod rgb_linear;
pub mod lch;
pub mod xyz;
pub mod lab;
pub mod yuv;
pub mod cmyk;
pub mod oklab;
pub mod oklch;
pub mod hwb;
pub mod ycbcr;
pub mod xyy;

/// Shared test helper: assert two colours match channel-by-channel within
/// `eps`, naming the channel that differs.
///
/// Every colour-space test module needs exactly this, and each one used to
/// carry its own byte-identical copy — thirteen of them. The per-space
/// `EPSILON` constants stay local, because they are genuinely different (CMYK
/// round-trips to 1e-7, YUV only to 1e-3) and that difference is part of what
/// each module is asserting.
#[cfg(test)]
pub(crate) fn assert_color_approx(c1: &super::Color, c2: &super::Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

/// Enumerates all supported color spaces for conversion and blending.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorSpace {
    /// Standard RGB with sRGB gamma curve.
    Srgb,
    /// Linear (scene-referred) RGB without gamma encoding.
    RgbLinear,
    /// Hue, Saturation, Lightness.
    Hsl,
    /// Hue, Saturation, Value.
    Hsv,
    /// CIE LCH (Lightness, Chroma, Hue) -- cylindrical form of Lab.
    Lch,
    /// CIE 1931 XYZ tristimulus values.
    Xyz,
    /// CIE L*a*b* perceptual color space.
    Lab,
    /// Luma and chrominance (analog video encoding).
    Yuv,
    /// Cyan, Magenta, Yellow, Key (black) -- subtractive color model.
    Cmyk,
    /// Oklab perceptual color space (uniform lightness; good for gradients).
    Oklab,
    /// Oklch -- cylindrical (L, chroma, hue) form of Oklab.
    Oklch,
    /// Hue, Whiteness, Blackness.
    Hwb,
    /// Digital YCbCr using Rec. 709 (HD) coefficients.
    Ycbcr,
    /// CIE xyY -- chromaticity (x, y) plus luminance (Y).
    Xyy,
}

impl ColorSpace {
    /// Returns an array of all 14 color space variants in display order.
    pub fn types() -> [ColorSpace; 14] {
        let types: [ColorSpace; 14] = [
            ColorSpace::Srgb,
            ColorSpace::RgbLinear,
            ColorSpace::Hsl,
            ColorSpace::Hsv,
            ColorSpace::Lch,
            ColorSpace::Xyz,
            ColorSpace::Lab,
            ColorSpace::Yuv,
            ColorSpace::Cmyk,
            ColorSpace::Oklab,
            ColorSpace::Oklch,
            ColorSpace::Hwb,
            ColorSpace::Ycbcr,
            ColorSpace::Xyy,
        ];

        types
    }
}