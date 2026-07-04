//! Edge detection and detail-enhancement filters.

/// Edge detection using Sobel-based convolution.
pub mod edge_detect;
/// Canny multi-stage edge detector with non-max suppression and hysteresis.
pub mod canny;
/// Difference of Gaussians / XDoG: stylized line-drawing filter.
pub mod dog;
/// Sharpening filter to enhance image detail.
pub mod sharpen;
/// Unsharp mask sharpening with configurable radius and threshold.
pub mod unsharpen;
/// Highpass filter: subtract a blurred copy from the original.
pub mod highpass;
/// Luminance-only highpass filter: preserves chroma.
pub mod luminance_highpass;
