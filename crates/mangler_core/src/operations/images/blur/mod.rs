//! Blur operations for the node graph.
//!
//! Provides various blur algorithms for softening and diffusing image details,
//! including standard Gaussian, directional, radial, slope-based, and non-uniform blurs.

/// Standard Gaussian blur with configurable radius.
pub mod blur;
/// Directional (motion) blur along a specified angle.
pub mod directional_blur;
/// Radial blur emanating from a center point.
pub mod radial_blur;
/// Slope-based blur using a height map to control blur direction.
pub mod slope_blur;
/// Non-uniform blur using a mask to control blur intensity per pixel.
pub mod non_uniform_blur;
