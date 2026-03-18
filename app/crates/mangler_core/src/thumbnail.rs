//! Thumbnail representations for node output previews.
//!
//! Thumbnails are generated from output values and sent to the UI for display
//! beneath each node on the graph canvas.

use image::{ImageBuffer, Rgba};

/// A small preview of a node's output value, shown in the graph editor.
#[derive(Debug, Clone, PartialEq)]
pub enum Thumbnail {
    /// A rasterized image thumbnail (e.g. for color swatches or image outputs).
    Image(ImageBuffer<Rgba<u8>, Vec<u8>>),
    /// A text-based thumbnail (e.g. for numeric, boolean, or string outputs).
    Text(String),
}