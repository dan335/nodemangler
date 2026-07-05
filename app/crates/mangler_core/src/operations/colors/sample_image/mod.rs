//! Image color sampling operations.
//!
//! Operations that analyze images to extract color information.

/// Finds the most frequently occurring colors in an image via HSL quantization.
pub mod most_common_colors;
/// Reads the color at a normalized (x, y) coordinate in an image.
pub mod sample_pixel;