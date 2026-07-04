//! Comparison operations that produce boolean results.
//!
//! All comparison nodes convert their inputs to `Decimal` for numeric comparison
//! (with booleans converting as `true` -> 1.0, `false` -> 0.0). Equality and
//! inequality operators also support direct `String` comparison without numeric
//! coercion.

/// Equality comparison (`a == b`).
pub mod equal;
/// Inequality comparison (`a != b`).
pub mod not_equal;
/// Tolerance-based equality comparison (`|a - b| <= tolerance`).
pub mod approx_equal;
/// Inclusive range membership test (`min <= value <= max`).
pub mod in_range;
/// Strict less-than comparison (`a < b`).
pub mod less_than;
/// Less-than-or-equal comparison (`a <= b`).
pub mod less_equal;
/// Strict greater-than comparison (`a > b`).
pub mod greater_than;
/// Greater-than-or-equal comparison (`a >= b`).
pub mod greater_equal;
