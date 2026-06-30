//! Video input (source) operations.

/// Loads a video from a file path on disk and outputs a specific frame.
pub mod file;
/// Downloads a video from a URL to a local cache file and emits a handle.
pub mod url;
