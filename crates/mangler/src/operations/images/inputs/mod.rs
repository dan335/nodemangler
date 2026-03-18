//! Image input (source) operations.
//!
//! Each submodule provides a node that produces an image from a different source:
//! loading from disk, fetching from a URL, pasting from the clipboard, generating
//! a solid color fill, or creating a linear gradient.

/// Loads an image from a URL using an async HTTP request.
pub mod url;
/// Loads an image from a file path on disk.
pub mod file;
/// Grabs an image from the system clipboard.
pub mod clipboard;
/// Generates a solid-color image of a specified size.
pub mod color;
/// Generates a vertical gradient image by blending two colors in a chosen color space.
pub mod gradient;
/// Renders a text string to a grayscale image using an embedded font.
pub mod text;