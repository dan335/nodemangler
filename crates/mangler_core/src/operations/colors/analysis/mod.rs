//! Color analysis operations (distance, luminance, contrast ratio, color temperature, dominant hue, harmony score, mix ratio).

/// CIE76 Delta E and Euclidean RGB distance between two colors.
pub mod distance;
/// BT.709 relative luminance of a color.
pub mod luminance;
/// WCAG contrast ratio between two colors.
pub mod contrast_ratio;
/// Perceptual color temperature estimation (McCamy formula).
pub mod color_temperature;
/// Dominant hue identification from a set of colors.
pub mod dominant_hue;
/// Color harmony score between two colors.
pub mod harmony_score;
/// Reverse-lerp mix ratio for colors.
pub mod mix_ratio;
