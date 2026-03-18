//! Text manipulation operations.
//!
//! These nodes transform, combine, or inspect `Text` values.

/// Appends two text values together.
pub mod append;
/// Returns the character count of a text value as an integer.
pub mod length;
/// Converts a text value to uppercase.
pub mod to_uppercase;
/// Converts a text value to lowercase.
pub mod to_lowercase;
/// Casts a `Text` value to the generic `String` type.
pub mod to_string;
