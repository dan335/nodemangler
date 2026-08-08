//! Decode dispatch for Libraries-panel thumbnails.
//!
//! Kept separate from [`super::library_thumbs`] (cache + worker pool) so the
//! "how do we get small RGBA pixels" path can evolve without touching queue
//! policy. Callers get opaque `(rgba8, w, h)` — never a `FloatImage` unless a
//! specialized decoder forces that intermediate.
//!
//! Ladder (cheapest first where it matters):
//! 1. Camera RAW → embedded preview JPEG via [`decode_raw_preview_rgba8`], else
//!    reduced [`decode_raw`].
//! 2. JPEG → `jpeg-decoder` scaled IDCT (not full-res decompress).
//! 3. jxl / psd / heic / heif → shared [`load_image_from_path`] + premultiplied resize.
//! 4. Everything else → `image` crate decode + `thumbnail`.

use std::fs::File;
use std::path::Path;

use mangler_core::operations::images::inputs::file::{is_raw_file, load_image_from_path};
use mangler_core::operations::images::inputs::raw_decode::{
    decode_raw, decode_raw_preview_rgba8, RawOptions,
};

/// Final resize / decode target (longest edge) for library thumbnails.
/// 192 covers HiDPI 80 pt cells without upscaling a soft 96 px texture.
pub const LIBRARY_THUMB_MAX: u32 = 192;

/// Longest-edge target for the reduced RAW *develop* fallback (when no usable
/// embedded preview exists). Headroom above [`LIBRARY_THUMB_MAX`] so a final
/// resize still looks clean.
const RAW_FALLBACK_MAX_DIMENSION: u32 = 512;

/// Decodes `path` into an RGBA8 thumbnail whose longest edge is ≤
/// [`LIBRARY_THUMB_MAX`]. Returns `(pixels, width, height)`.
pub fn decode_thumb(path: &Path) -> Result<(Vec<u8>, [usize; 2]), ()> {
    if is_raw_file(path) {
        return decode_raw_thumb(path);
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    match ext.as_deref() {
        Some("jpg" | "jpeg") => decode_jpeg_scaled(path),
        Some("jxl" | "psd" | "heic" | "heif") => decode_via_float_image(path),
        _ => decode_image_crate(path),
    }
}

fn decode_raw_thumb(path: &Path) -> Result<(Vec<u8>, [usize; 2]), ()> {
    // Prefer the camera's embedded preview (no demosaic). Library thumbs are
    // browsing aids — matching the `from raw` node pixel-for-pixel is not required.
    if let Ok((rgba, w, h)) = decode_raw_preview_rgba8(path, LIBRARY_THUMB_MAX) {
        return Ok((rgba, [w as usize, h as usize]));
    }

    let image = decode_raw(
        path,
        &RawOptions {
            max_dimension: Some(RAW_FALLBACK_MAX_DIMENSION),
            ..RawOptions::default()
        },
    )
    .map_err(|_| ())?;
    let thumb = image.resize_fit_premultiplied(LIBRARY_THUMB_MAX, LIBRARY_THUMB_MAX);
    let rgba = thumb.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok((rgba.into_raw(), size))
}

/// JPEG via `jpeg-decoder` with scaled IDCT so a 24 MP file never fully expands.
fn decode_jpeg_scaled(path: &Path) -> Result<(Vec<u8>, [usize; 2]), ()> {
    use jpeg_decoder::{Decoder, PixelFormat};

    let file = File::open(path).map_err(|_| ())?;
    let mut decoder = Decoder::new(file);

    // Request a decode no larger than LIBRARY_THUMB_MAX on either edge; the
    // decoder picks 1/1, 1/2, 1/4, or 1/8 IDCT scale.
    let target = LIBRARY_THUMB_MAX.min(u16::MAX as u32) as u16;
    let (_scaled_w, _scaled_h) = decoder.scale(target, target).map_err(|_| ())?;
    let pixels = decoder.decode().map_err(|_| ())?;
    let info = decoder.info().ok_or(())?;

    let w = info.width as u32;
    let h = info.height as u32;
    let rgba = match info.pixel_format {
        PixelFormat::L8 => {
            let mut out = Vec::with_capacity(pixels.len() * 4);
            for g in pixels {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        PixelFormat::RGB24 => {
            let mut out = Vec::with_capacity((pixels.len() / 3) * 4);
            for chunk in pixels.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        PixelFormat::CMYK32 => {
            // Approximate CMYK→RGB for thumbs only (not color-managed).
            let mut out = Vec::with_capacity((pixels.len() / 4) * 4);
            for chunk in pixels.chunks_exact(4) {
                let (c, m, y, k) = (
                    chunk[0] as u16,
                    chunk[1] as u16,
                    chunk[2] as u16,
                    chunk[3] as u16,
                );
                let r = ((255 - c) * (255 - k) / 255) as u8;
                let g = ((255 - m) * (255 - k) / 255) as u8;
                let b = ((255 - y) * (255 - k) / 255) as u8;
                out.extend_from_slice(&[r, g, b, 255]);
            }
            out
        }
        PixelFormat::L16 => return Err(()), // rare; fall through not available here
    };

    // Scaled IDCT may still be larger than LIBRARY_THUMB_MAX (e.g. 1/4 of 8000
    // = 2000). Final shrink with the image crate.
    if w.max(h) > LIBRARY_THUMB_MAX {
        let dyn_img = image::RgbaImage::from_raw(w, h, rgba).ok_or(())?;
        let thumb = image::DynamicImage::ImageRgba8(dyn_img).thumbnail(LIBRARY_THUMB_MAX, LIBRARY_THUMB_MAX);
        let rgba = thumb.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        return Ok((rgba.into_raw(), size));
    }

    Ok((rgba, [w as usize, h as usize]))
}

fn decode_via_float_image(path: &Path) -> Result<(Vec<u8>, [usize; 2]), ()> {
    let image = load_image_from_path(path).map_err(|_| ())?;
    let thumb = image.resize_fit_premultiplied(LIBRARY_THUMB_MAX, LIBRARY_THUMB_MAX);
    let rgba = thumb.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok((rgba.into_raw(), size))
}

fn decode_image_crate(path: &Path) -> Result<(Vec<u8>, [usize; 2]), ()> {
    let reader = image::ImageReader::open(path)
        .map_err(|_| ())?
        .with_guessed_format()
        .map_err(|_| ())?;
    let dyn_img = reader.decode().map_err(|_| ())?;
    let thumb = dyn_img.thumbnail(LIBRARY_THUMB_MAX, LIBRARY_THUMB_MAX);
    let rgba = thumb.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok((rgba.into_raw(), size))
}

/// Extension of `path` lowercased, for pure unit tests of the dispatch table.
#[cfg(test)]
pub fn thumb_dispatch_kind(path: &Path) -> &'static str {
    if is_raw_file(path) {
        return "raw";
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "jpeg_scaled",
        Some("jxl" | "psd" | "heic" | "heif") => "float_image",
        _ => "image_crate",
    }
}

#[cfg(test)]
#[path = "thumb_decode_tests.rs"]
mod tests;
