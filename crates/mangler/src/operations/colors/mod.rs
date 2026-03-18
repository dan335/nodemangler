//! Color operations for the node graph.
//!
//! This module organizes color-related operations into submodules for creating
//! colors from various color spaces, converting colors to different color spaces,
//! blending colors, and sampling colors from images.

/// Operations for constructing colors from individual channel values in various color spaces.
pub mod inputs;
/// Operations for decomposing colors into individual channel values in various color spaces.
pub mod outputs;
/// Operations for blending two colors together.
pub mod blend;
/// Operations for analyzing and sampling colors from images.
pub mod sample_image;
/// Cast operations for converting values to colors.
pub mod cast;