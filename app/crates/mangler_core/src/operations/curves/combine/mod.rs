//! Curve combiner nodes: take two curves in, produce one curve out.
//!
//! `join` concatenates end-to-end (control points when both curves share an
//! interpolation, flattened polylines otherwise); `morph` resamples both to a
//! matching point count and linearly interpolates between them. Both fall
//! back to returning the other input unchanged when one side is degenerate
//! (fewer than 2 points) — never error, never emit an empty curve.

pub mod join;
pub mod morph;
