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
}

impl ColorSpace {
    /// Returns an array of all 9 color space variants in display order.
    pub fn types() -> [ColorSpace; 9] {
        let types: [ColorSpace; 9] = [
            ColorSpace::Srgb,
            ColorSpace::RgbLinear,
            ColorSpace::Hsl,
            ColorSpace::Hsv,
            ColorSpace::Lch,
            ColorSpace::Xyz,
            ColorSpace::Lab,
            ColorSpace::Yuv,
            ColorSpace::Cmyk,
        ];

        types
    }
}