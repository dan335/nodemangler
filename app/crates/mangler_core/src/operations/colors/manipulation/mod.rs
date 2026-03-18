//! Color manipulation operations (invert, grayscale, HSV adjustment, clamp, set alpha).

/// Invert RGB channels.
pub mod invert;
/// Convert to grayscale via BT.709 luminance.
pub mod grayscale;
/// Adjust hue, saturation, and value offsets.
pub mod adjust_hsv;
/// Clamp RGB channels to a min/max range.
pub mod clamp;
/// Replace or multiply the alpha channel.
pub mod set_alpha;
