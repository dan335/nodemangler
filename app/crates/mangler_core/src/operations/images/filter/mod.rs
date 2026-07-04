//! Filter operations for the node graph.
//!
//! Provides convolution-based image filters for detecting edges, creating
//! emboss effects, and sharpening images.

/// Generic 3x3 convolution with divisor and bias.
pub mod convolution;

/// Edge detection and detail-enhancement filters.
pub mod edges;
/// Edge-preserving smoothing and denoising filters.
pub mod smoothing;
/// Morphological operators (erode/dilate family).
pub mod morphology;
/// Painterly and graphic stylization filters.
pub mod stylize;
/// Dithering filters.
pub mod dither;
