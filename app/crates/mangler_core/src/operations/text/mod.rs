//! Text operations for the node graph.
//!
//! Provides operations for producing and manipulating `Text` values.

/// Text input (source) nodes.
pub mod inputs;
/// Text manipulation nodes (append, length, case, cast).
pub mod manipulation;
/// Image → text operations (ASCII art, data URI, info, palette, hash).
pub mod image;
/// Text encoding/decoding operations (Base64, URL percent-encoding).
pub mod encoding;
