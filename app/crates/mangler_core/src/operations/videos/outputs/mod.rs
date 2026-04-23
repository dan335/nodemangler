//! Video output (sink) operations.

/// Writes a video file. Rendering itself happens on a separate task in the
/// engine; this operation's `run()` is a no-op passthrough so thumbnails
/// keep updating interactively.
pub mod file;
