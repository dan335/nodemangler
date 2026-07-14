//! Curve measurement operations that reduce a [`crate::curve::Curve`] to
//! numbers.
//!
//! These nodes take a `Value::Curve` and emit numeric outputs — length, point
//! count, bounds, centroid, area, and arc-length samples — so a curve can
//! drive the numeric side of the graph (spacing math, thresholds, per-point
//! stamping). They live under the `numbers` category because their outputs
//! are numbers, even though their input is a curve (the repo's
//! categorize-by-output-type convention, same reasoning as
//! [`crate::operations::numbers::image`]).
//!
//! All values are in the curve's own normalized `[0,1]²` units. Degenerate
//! curves (fewer than 2 points, or empty) never error — each node falls back
//! to a documented default (0s, or the geometric center `(0.5, 0.5)`).

/// Total arc length of the flattened curve.
pub mod length;
/// Control-point count (not the flattened sample count).
pub mod point_count;
/// Axis-aligned bounding box of the flattened curve.
pub mod bounds;
/// Arc-length-weighted centroid of the flattened curve.
pub mod centroid;
/// Enclosed area (absolute and signed) of the implicitly-closed curve.
pub mod area;
/// Position and tangent angle at a normalized arc-length parameter.
pub mod sample_point;
