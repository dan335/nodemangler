//! Image adjustment operations for pixel-level color and tone manipulation.
//!
//! This module contains operations that modify image appearance without changing
//! dimensions: contrast, levels, color grading, and more.

pub mod auto_levels;
pub mod brighten;
pub mod color_match;
pub mod contrast;
pub mod curves;
pub mod distance;
pub mod dither;
pub mod gradient_dynamic;
pub mod gradient_map;
pub mod grayscale;
pub mod histogram_range;
pub mod histogram_scan;
pub mod histogram_select;
pub mod hue_rotate;
pub mod invert;
pub mod levels;
pub mod posterize;
