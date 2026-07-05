//! Text → number operations: measure or parse a `Text` value into a number.
//!
//! These nodes take `Text` and emit an `Integer`/`Decimal`, so they live under
//! the `numbers` category (categorized by output type) even though their input
//! is text — the number counterpart to `numbers/image/`. (The existing
//! `text/manipulation/length` node predates this convention and still lives
//! under text.)

/// Parses text into a decimal (errors if unparseable).
pub mod parse_decimal;
/// Parses text into an integer (errors if unparseable).
pub mod parse_integer;
/// Counts whitespace-separated words.
pub mod word_count;
/// Counts lines (newline-separated).
pub mod line_count;
/// Counts UTF-8 bytes (distinct from `length`, which counts characters).
pub mod byte_length;
/// Finds the character index of the first occurrence of a substring (−1 if absent).
pub mod index_of;
/// Counts non-overlapping occurrences of a substring.
pub mod count_occurrences;
