//! Shape generation operations.
//!
//! This module provides operations that generate geometric shapes as grayscale
//! SDF (signed distance field) images. Each shape is anti-aliased using smoothstep
//! and supports rotation where applicable. Available shapes include rectangle,
//! ellipse, polygon, star, and line.

pub mod circle;
pub mod cone;
pub mod curve_distance_field;
pub mod curve_gradient;
pub mod ellipse;
pub mod line;
pub mod paraboloid;
pub mod polygon;
pub mod pyramid;
pub mod rasterize_curve;
pub mod rectangle;
pub mod star;
