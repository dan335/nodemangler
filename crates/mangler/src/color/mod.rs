//! Color representation and manipulation.
//!
//! Provides the core [`Color`] type stored internally as sRGBA floats,
//! along with conversions to 9 color spaces and 17 blend modes.

use serde::{Serialize, Deserialize};

pub mod blend;
pub mod color_spaces;

/// An RGBA color stored as sRGB floats.
///
/// Each channel is nominally in the range `0.0..=1.0`, though values outside
/// that range are permitted for intermediate calculations. Conversions to
/// other color spaces are provided via methods in the [`color_spaces`] module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub struct Color {
    /// Red channel (sRGB, 0.0 - 1.0).
    pub r: f32,
    /// Green channel (sRGB, 0.0 - 1.0).
    pub g: f32,
    /// Blue channel (sRGB, 0.0 - 1.0).
    pub b: f32,
    /// Alpha (opacity) channel (0.0 - 1.0).
    pub a: f32,
}