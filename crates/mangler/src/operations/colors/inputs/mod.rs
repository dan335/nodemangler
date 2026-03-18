//! Color input operations.
//!
//! Each submodule provides a node that constructs a [`Color`](crate::color::Color)
//! from individual channel values in a specific color space.

/// Construct a color from CMYK (Cyan, Magenta, Yellow, Key/Black) + alpha channels.
pub mod cmyk;
/// Construct a color from HSL (Hue, Saturation, Lightness) + alpha channels.
pub mod hsl;
/// Construct a color from HSV (Hue, Saturation, Value) + alpha channels.
pub mod hsv;
/// Construct a color from CIE L*a*b* + alpha channels.
pub mod lab;
/// Construct a color from LCH (Lightness, Chroma, Hue) + alpha channels.
pub mod lch;
/// Construct a color from linear RGB + alpha channels (no gamma curve).
pub mod rgb_linear;
/// Construct a color from sRGB + alpha channels (gamma-encoded).
pub mod srgb;
/// Construct a color from CIE XYZ + alpha channels.
pub mod xyz;
/// Construct a color from YUV (luminance + chrominance) + alpha channels.
pub mod yuv;
