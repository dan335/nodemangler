//! Curve generator nodes: procedural shapes and paths emitted as a
//! [`crate::curve::Curve`] rather than drawn by hand.
//!
//! All parameters are in normalized `[0,1]²` curve space (y-down), independent
//! of any raster resolution — the same node output maps onto any image size.
//! Degenerate inputs (zero radius, too few sides, etc.) are floored to still
//! emit a valid curve with at least 2 points; these nodes never error and
//! never emit an empty curve.

pub mod arc;
pub mod ellipse;
pub mod fractal_line;
pub mod lissajous;
pub mod polygon;
pub mod random_walk;
pub mod spiral;
pub mod star;
pub mod superellipse;
pub mod wave;
