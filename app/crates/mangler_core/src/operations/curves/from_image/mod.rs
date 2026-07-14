//! Curve-from-image operations: extract [`crate::curve::Curve`] geometry from
//! raster images. Categorized under `curves` by the output-type rule — every
//! op here emits a `Value::Curve` even though it consumes an image.

pub mod trace_contour;
