//! Shape generation operations.
//!
//! This module provides operations that generate geometric shapes as grayscale
//! SDF (signed distance field) images. Each shape is anti-aliased using smoothstep
//! and supports rotation where applicable. Available shapes include rectangle,
//! ellipse, polygon, star, and line.

pub mod rectangle;
pub mod ellipse;
pub mod polygon;
pub mod star;
pub mod line;
