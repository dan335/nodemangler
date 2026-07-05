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
/// Joins multiple text values with a separator.
pub mod join;
/// Replaces occurrences of a substring with another.
pub mod replace;
/// Extracts a substring by character start + length.
pub mod substring;
/// Splits text by a delimiter, returning a chosen field and the remainder.
pub mod split;
/// Trims leading/trailing whitespace (or given characters).
pub mod trim;
/// Pads text to a target width with a fill string.
pub mod pad;
/// Repeats text a given number of times.
pub mod repeat;
/// Reverses the characters of a text value.
pub mod reverse;
/// Substitutes `{}` placeholders in a template with input values.
pub mod template;
/// Converts text to Title Case.
pub mod title_case;
/// Formats a number as text with fixed decimals and minimum width.
pub mod format_number;
