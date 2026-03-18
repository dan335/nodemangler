//! Image output (export) operations.
//!
//! Submodules provide nodes for writing images to external destinations:
//! saving to a file on disk or copying to the system clipboard.

/// Saves an image to a file in a configurable format (JPEG, PNG, etc.).
pub mod file;
/// Copies an image to the system clipboard.
pub mod clipboard;