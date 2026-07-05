//! Text → boolean predicates.
//!
//! These nodes take `Text` and emit a `Bool`, so they live under the `logic`
//! category (categorized by output type) even though their input is text.

/// True if the text contains a substring.
pub mod contains;
/// True if the text starts with a prefix.
pub mod starts_with;
/// True if the text ends with a suffix.
pub mod ends_with;
/// True if the text is empty (or whitespace-only, optionally).
pub mod is_empty;
/// True if two strings are equal ignoring ASCII case.
pub mod equals_ignore_case;
