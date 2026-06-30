//! Color output (decomposition) operations.
//!
//! Each submodule provides a node that converts a [`Color`](crate::color::Color)
//! into individual channel values in a specific color space.

/// Decompose a color into CMYK (Cyan, Magenta, Yellow, Key/Black) + alpha channels.
pub mod to_cmyk;
/// Decompose a color into HSL (Hue, Saturation, Lightness) + alpha channels.
pub mod to_hsl;
/// Decompose a color into HSV (Hue, Saturation, Value) + alpha channels.
pub mod to_hsv;
/// Decompose a color into CIE L*a*b* + alpha channels.
pub mod to_lab;
/// Decompose a color into LCH (Lightness, Chroma, Hue) + alpha channels.
pub mod to_lch;
/// Decompose a color into linear RGB + alpha channels (no gamma curve).
pub mod to_rgb_linear;
/// Decompose a color into sRGB + alpha channels (gamma-encoded).
pub mod to_srgb;
/// Decompose a color into CIE XYZ + alpha channels.
pub mod to_xyz;
/// Decompose a color into YUV (luminance + chrominance) + alpha channels.
pub mod to_yuv;
/// Decompose a color into Oklab (perceptual L, a, b) + alpha channels.
pub mod to_oklab;
/// Decompose a color into Oklch (perceptual L, chroma, hue) + alpha channels.
pub mod to_oklch;
/// Decompose a color into HWB (Hue, Whiteness, Blackness) + alpha channels.
pub mod to_hwb;
/// Decompose a color into YCbCr (BT.709 luma + chroma) + alpha channels.
pub mod to_ycbcr;
/// Decompose a color into CIE xyY (chromaticity + luminance) + alpha channels.
pub mod to_xyy;