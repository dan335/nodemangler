//! Interpolation and mapping operations for the node graph.
//!
//! Provides functions for smoothly transitioning between values, including
//! linear interpolation, smooth step functions, and range remapping.

/// Linearly interpolates between two values.
pub mod lerp;
/// Smooth Hermite interpolation between two edges.
pub mod smoothstep;
/// Returns 0 if input < edge, 1 otherwise.
pub mod step;
/// Remaps a value from one range to another.
pub mod map_range;
