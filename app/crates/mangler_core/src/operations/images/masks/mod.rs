//! Mask generation and combination operations.
//!
//! Produces single-channel linear masks for local photo adjustments and
//! material layering. Geometric generators (`linear gradient`, `radial
//! gradient`) take explicit width/height like the shape nodes; parametric
//! selectors (`hue range`) take a source image; `mask combine` merges two
//! masks with set-logic ops.

pub mod linear_gradient;
pub mod radial_gradient;
pub mod hue_range;
pub mod combine;
