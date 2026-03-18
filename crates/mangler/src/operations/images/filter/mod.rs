//! Filter operations for the node graph.
//!
//! Provides convolution-based image filters for detecting edges, creating
//! emboss effects, and sharpening images.

/// Edge detection using Sobel-based convolution.
pub mod edge_detect;
/// Emboss effect using directional convolution.
pub mod emboss;
/// Sharpening filter to enhance image detail.
pub mod sharpen;
/// Unsharp mask sharpening with configurable radius and threshold.
pub mod unsharpen;
