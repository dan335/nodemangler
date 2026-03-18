//! Image adjustment operations for pixel-level color and tone manipulation.
//!
//! This module contains operations that modify image appearance without changing
//! dimensions: blur, contrast, levels, color grading, edge detection, and more.

pub mod auto_levels;
pub mod blur;
pub mod contrast;
pub mod curves;
pub mod directional_blur;
pub mod distance;
pub mod edge_detect;
pub mod emboss;
pub mod gradient_map;
pub mod grayscale;
pub mod histogram_range;
pub mod histogram_scan;
pub mod invert;
pub mod brighten;
pub mod hue_rotate;
pub mod levels;
pub mod non_uniform_blur;
pub mod posterize;
pub mod radial_blur;
pub mod sharpen;
pub mod slope_blur;
pub mod unsharpen;