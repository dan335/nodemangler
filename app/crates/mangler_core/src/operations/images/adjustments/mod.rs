//! Image adjustment operations for pixel-level color and tone manipulation.
//!
//! This module contains operations that modify image appearance without changing
//! dimensions: contrast, levels, color grading, and more.

pub(crate) mod common;

pub mod auto_levels;
pub mod black_white;
pub mod brighten;
pub mod clarity;
pub mod color_balance;
pub mod color_lookup;
pub mod color_match;
pub mod color_to_mask;
pub mod contrast;
pub mod curves;
pub mod dehaze;
pub mod distance;
pub mod dither;
pub mod exposure;
pub mod frequency_split;
pub mod gradient_dynamic;
pub mod gradient_map;
pub mod grain;
pub mod grayscale;
pub mod histogram_range;
pub mod histogram_scan;
pub mod histogram_select;
pub mod hsl;
pub mod hue_rotate;
pub mod invert;
pub mod levels;
pub mod photo_filter;
pub mod posterize;
pub mod replace_color;
pub mod saturation;
pub mod selective_color;
pub mod threshold;
pub mod vibrance;
pub mod vignette;
pub mod white_balance;
