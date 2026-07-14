//! Curve modifier nodes: take a curve in, produce a modified curve out.
//!
//! Two behavior classes, stated in each node's help text:
//! - **Structure-preserving** (transform, mirror, reverse) — operate directly
//!   on `points` and `handles` (as vectors), preserving `interpolation` and
//!   `closed` exactly.
//! - **Flattening** (simplify, resample, jitter, offset, trim, round_corners,
//!   smooth) — flatten to an `f64` polyline (or, for smooth/round_corners,
//!   work on control points directly), do the geometry in `f64`, and emit a
//!   `Linear` curve capped at `common::MAX_OUTPUT_POINTS` points.
//!
//! All length-like parameters (tolerance, spacing, jitter amount, offset
//! distance, corner radius) are authored as pixels at a 1024px reference and
//! divided by 1024 into normalized curve-space units — a unit convention
//! only, since curves have no raster size (unlike `scale_to_resolution`,
//! which scales to an actual image).
//!
//! Universal edge case: an input curve with fewer than 2 points passes
//! through unchanged — these nodes never error and never emit an empty curve.

pub mod jitter;
pub mod mirror;
pub mod offset;
pub mod resample;
pub mod reverse;
pub mod round_corners;
pub mod simplify;
pub mod smooth;
pub mod transform;
pub mod trim;
