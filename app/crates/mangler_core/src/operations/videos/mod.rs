//! Video operations for the node graph engine.
//!
//! Decoded and encoded via the [`video`](crate::video) module (gated behind
//! the `video` cargo feature).

/// Video source operations: load a video file (emits a `Value::Video` handle).
pub mod inputs;
/// Video sink operations: render the graph out to a video file.
pub mod outputs;
/// Video→image transforms: extract individual frames by index or time.
pub mod transform;
