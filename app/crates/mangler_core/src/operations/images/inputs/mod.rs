//! Image input (source) operations.
//!
//! Each submodule provides a node that produces an image from a different source:
//! loading from disk, fetching from a URL, pasting from the clipboard, generating
//! a solid color fill, or creating a linear gradient.
//!
//! ## Embedding pixels in the graph file
//! Most sources here are *references* — a path, a folder, a URL — so a saved
//! graph reproduces them by re-reading. The clipboard is not: whatever was on
//! it when the node ran is gone by the next session. [`encode_png_base64`] /
//! [`decode_png_base64`] are the format such a node stores its pixels in, via
//! [`Input::embedded_image`](crate::input::Input::embedded_image).

use crate::float_image::FloatImage;
use crate::operations::text::encoding::{base64_decode, base64_encode};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

/// Loads an image from a URL using an async HTTP request.
pub mod url;
/// Loads an image from a file path on disk.
pub mod file;
/// Decodes camera RAW files (Canon CR3/CR2, Nikon NEF, Sony ARW, DNG, …) via rawler.
pub mod raw_decode;
/// Develops a camera RAW file with control over white balance, encoding and size.
pub mod raw;
/// Loads one image at a time from a folder of images, selected by index.
pub mod from_folder;
/// Grabs an image from the system clipboard.
pub mod clipboard;
/// Generates a solid-color image of a specified size.
pub mod color;
/// Generates a vertical gradient image by blending two colors in a chosen color space.
pub mod gradient;
/// Renders a text string to a grayscale image using an embedded font.
pub mod text;
/// Generates a solid grayscale image from a single constant value.
pub mod constant;

/// Encode an image as a base64 PNG for storage inside the graph file.
///
/// **8-bit, and deliberately so.** The one caller is `from clipboard`, whose
/// pixels arrived as RGBA8 in the first place, so the round-trip is exact.
/// Keeping 32-bit float precision would mean a format the `image` crate can
/// write losslessly (TIFF/EXR) at several times the size, for no gain on a
/// source that never had the range.
pub(crate) fn encode_png_base64(image: &FloatImage) -> Result<String, String> {
    let dynamic = DynamicImage::ImageRgba8(image.to_rgba8());
    let mut bytes: Vec<u8> = Vec::new();
    dynamic
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| format!("Failed to encode the image as PNG: {e}"))?;
    Ok(base64_encode(&bytes))
}

/// Exact inverse of [`encode_png_base64`].
///
/// Errors rather than falling back to a placeholder: a graph file whose
/// embedded image is corrupt should say so, not quietly show a blank frame that
/// the user then re-saves over the top of the real data.
pub(crate) fn decode_png_base64(encoded: &str) -> Result<FloatImage, String> {
    let bytes = base64_decode(encoded)
        .ok_or_else(|| "Stored image is not valid base64.".to_string())?;
    let dynamic = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
        .map_err(|e| format!("Stored image could not be decoded: {e}"))?;
    Ok(FloatImage::from_dynamic(&dynamic))
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;