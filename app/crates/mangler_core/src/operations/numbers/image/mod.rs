//! Image measurement operations that reduce an image to numbers.
//!
//! These nodes take a `Value::Image` and emit numeric outputs — dimensions,
//! statistics, and content measures — so an image can drive the numeric side
//! of the graph (resize math, thresholds, conditionals). They live under the
//! `numbers` category because their outputs are numbers, even though their
//! input is an image. (The color-producing `sample pixel` node lives under
//! `colors/sample_image/` instead.)
//!
//! Luminance-based reductions share the [`pixel_luma`]/[`luma_values`]
//! helpers, and channel-expansion shares [`pixel_rgba`], so every node agrees
//! on how a pixel of any channel count collapses to brightness or RGBA.

use crate::float_image::FloatImage;

/// Image width, height, aspect ratio, and channel count as numbers.
pub mod dimensions;
/// Per-channel and luminance mean of every pixel.
pub mod mean;
/// Axis-aligned bounding box of content above a threshold.
pub mod bounding_box;
/// Luminance-weighted centroid (center of mass).
pub mod centroid;
/// Fraction of pixels whose significance exceeds a threshold.
pub mod coverage;
/// Minimum, maximum, and range of pixel luminance.
pub mod min_max;
/// Median pixel luminance.
pub mod median;
/// Luminance value at a given percentile.
pub mod percentile;
/// Standard deviation and variance of pixel luminance.
pub mod std_dev;
/// Shannon entropy of the luminance histogram.
pub mod entropy;
/// Skewness (third standardized moment) of luminance.
pub mod skewness;
/// Kurtosis (fourth standardized moment) of luminance.
pub mod kurtosis;
/// Focus measure via the variance of the Laplacian.
pub mod sharpness;
/// Fraction of pixels that sit on an edge (Sobel magnitude threshold).
pub mod edge_density;
/// Count of distinct quantized colors.
pub mod unique_colors;
/// Saturation-weighted circular-mean hue, mean saturation, and value.
pub mod average_hue;
/// Similarity metrics (MSE/RMSE/MAE/PSNR) between two images.
pub mod image_difference;
/// Perceptual-hash (dHash) Hamming distance / similarity between two images.
pub mod perceptual_hash;

/// Rec. 601 luminance of a single pixel slice (1–4 channels).
///
/// - 1 or 2 channels: the first (grayscale) channel; alpha ignored
/// - 3/4 channels: `0.299 R + 0.587 G + 0.114 B`; alpha ignored
#[inline]
pub(crate) fn pixel_luma(px: &[f32]) -> f32 {
    match px.len() {
        0 => 0.0,
        1 | 2 => px[0],
        _ => 0.299 * px[0] + 0.587 * px[1] + 0.114 * px[2],
    }
}

/// Expands a pixel slice of any channel count to `(r, g, b, a)`.
///
/// Grayscale channels are replicated across RGB; a missing alpha defaults to 1.
#[inline]
pub(crate) fn pixel_rgba(px: &[f32]) -> (f32, f32, f32, f32) {
    match px.len() {
        0 => (0.0, 0.0, 0.0, 1.0),
        1 => (px[0], px[0], px[0], 1.0),
        2 => (px[0], px[0], px[0], px[1]),
        3 => (px[0], px[1], px[2], 1.0),
        _ => (px[0], px[1], px[2], px[3]),
    }
}

/// Collects the per-pixel Rec. 601 luminance for every pixel in the image.
pub(crate) fn luma_values(img: &FloatImage) -> Vec<f32> {
    img.pixels().map(pixel_luma).collect()
}
