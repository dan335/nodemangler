//! Image output (export) operations.
//!
//! Submodules provide nodes for writing images to external destinations:
//! saving to a file on disk or copying to the system clipboard.

/// Saves an image to a file in a configurable format (JPEG, PNG, etc.).
pub mod file;
/// Copies an image to the system clipboard.
pub mod clipboard;
/// Exports channel-packed PBR textures for a game engine (Godot/Unity/Unreal/Custom).
pub mod material;
/// Channel-packing specs, parser, and engine backing the material export node.
pub mod material_presets;

use image::ImageFormat;
use image::codecs::avif::AvifEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType as PngCompression, FilterType as PngFilter, PngEncoder};
use image::{DynamicImage, ImageBuffer};
use crate::float_image::FloatImage;
use crate::input::Input;
use crate::value::{ColorFormat, Value};
use std::io::{BufWriter, Write};

/// Decide whether an output node (`to file`, `material`, `to clipboard`) should
/// actually write on this run, and consume the one-shot manual-save pulse.
///
/// `auto_save_idx` holds the `Value::Bool` auto-save toggle (off by default);
/// `save_idx` holds the `Value::Bool` fired by the manual "save" button.
/// `honor_force` controls whether the engine's headless force-save flag (see
/// below) counts at all — `to file`/`material` always pass `true` so a
/// headless `graph.run()` still emits files with no user around to click
/// "save", but `to clipboard` passes `false` while a batch run is iterating
/// (see [`crate::run_context::RunContext::batch_item_stem`]) so a 200-image
/// batch doesn't rewrite the clipboard 200 times over. A write happens when
/// **any** of these is true:
/// - auto-save is on,
/// - the save button was just clicked (the pulse is `true`), or
/// - `honor_force` is true and the engine forced saving for this run (a
///   headless CLI `graph.run()`, via [`crate::run_context`]).
///
/// The save pulse is reset to `false` here so a click writes exactly once: a
/// later reactive run (e.g. the image input changing) then sees `false` and
/// does not re-save. Input mutations persist back onto the node (see
/// [`Node::run`](crate::node::Node::run)), and `Graph::set_input` clears the
/// input-hash cache so a repeat click re-fires. The pulse is left untouched when
/// the input is driven by a connection (the upstream value owns it).
pub(crate) fn should_save_and_consume(
    inputs: &mut [Input],
    auto_save_idx: usize,
    save_idx: usize,
    honor_force: bool,
) -> bool {
    let auto = matches!(inputs.get(auto_save_idx).map(|i| &i.value), Some(Value::Bool(true)));
    let pulse = matches!(inputs.get(save_idx).map(|i| &i.value), Some(Value::Bool(true)));

    if let Some(input) = inputs.get_mut(save_idx) {
        if input.connection.is_none() {
            input.value = Value::Bool(false);
        }
    }

    let forced = honor_force && crate::run_context::current().map(|c| c.force_save).unwrap_or(false);
    auto || pulse || forced
}

/// Build the standard pair of save-gating inputs appended to every image output
/// node: an `auto save` checkbox (off by default) and a momentary `save`
/// button. Kept here so the three output ops stay in lockstep.
pub(crate) fn save_gate_inputs() -> [Input; 2] {
    [
        Input::new("auto save".to_string(), Value::Bool(false), None, None)
            .with_description(
                "When on, this node writes every time its input changes. When off (the default), \
                 nothing is written until you press the save button.",
            ),
        Input::new("save".to_string(), Value::Bool(false), Some(crate::input::InputSettings::Button), None)
            .with_description("Write the current image once now (used when auto save is off)."),
    ]
}

/// Resolve an output node's `folder` + `file name` inputs into a concrete
/// destination directory and a sanitized file stem, using the graph's
/// [`run_context`](crate::run_context) for the base directory, default name,
/// and in-progress batch item.
///
/// Shared by the `to file` and `material` nodes so both resolve paths
/// identically: an absolute folder is used as-is, a relative one joins the
/// graph's own directory, and an empty one means the graph's directory itself.
/// The file name follows this rule (evaluated before the one-time
/// [`sanitize_name`](crate::naming::sanitize_name) pass, so the composed
/// result is sanitized exactly once):
/// - a non-empty, **connected** `file_name` (e.g. wired from a from-folder
///   node's `file name` output) is used verbatim — the user explicitly chose
///   this name per-run, so it is never decorated, batch or not;
/// - a non-empty, unwired `file_name` is used as-is outside a batch run, but
///   gets `_{item}` appended during a batch iteration — an unwired name is a
///   template typed once (often pre-filled with the graph's own name) and
///   reused by every output node on every iteration, so without the suffix
///   every iteration would overwrite the last;
/// - an empty `file_name` falls back to the batch item's stem during a batch
///   iteration, or the graph's name otherwise, exactly as before batching
///   existed.
///
/// Does **not** touch the filesystem — the caller creates the directory when
/// it is ready to write. `folder_idx` / `file_name_idx` are used only to
/// attribute errors to the right input.
pub(crate) fn resolve_output_dir_and_stem(
    folder: &std::path::Path,
    file_name: &str,
    folder_idx: usize,
    file_name_idx: usize,
    file_name_connected: bool,
) -> Result<(std::path::PathBuf, String), crate::operations::OperationError> {
    use crate::operations::OperationError;

    let ctx = crate::run_context::current().unwrap_or_default();

    // Absolute stays as-is; relative joins the graph's folder; empty means the
    // graph's folder itself.
    let resolved_dir = if folder.as_os_str().is_empty() {
        ctx.graph_dir.clone().unwrap_or_default()
    } else if folder.is_absolute() {
        folder.to_path_buf()
    } else {
        match &ctx.graph_dir {
            Some(dir) => dir.join(folder),
            None => folder.to_path_buf(),
        }
    };
    if resolved_dir.as_os_str().is_empty() {
        let msg = "No folder set and the graph has no save location yet.".to_string();
        return Err(OperationError { input_errors: vec![(folder_idx, msg.clone())], node_error: Some(msg) });
    }

    // Compose the raw (unsanitized) stem per the rule documented above.
    let raw_stem = if file_name.trim().is_empty() {
        match &ctx.batch_item_stem {
            Some(item) => item.clone(),
            None => ctx.graph_name.clone(),
        }
    } else if file_name_connected {
        file_name.to_string()
    } else {
        match &ctx.batch_item_stem {
            Some(item) => format!("{file_name}_{item}"),
            None => file_name.to_string(),
        }
    };

    // Sanitize the composed name exactly once.
    let stem = crate::naming::sanitize_name(&raw_stem);
    if stem.is_empty() {
        let msg = "File name is empty (and the graph has no name to fall back to).".to_string();
        return Err(OperationError { input_errors: vec![(file_name_idx, msg.clone())], node_error: Some(msg) });
    }

    Ok((resolved_dir, stem))
}

// --- Component conversions --------------------------------------------------
//
// The save path previously went `FloatImage::to_dynamic()` (1/2ch → Luma16 /
// LumaA16 by truncation, 3/4ch → Rgb32F/Rgba32F) followed by the `image`
// crate's whole-buffer conversions, allocating up to three full-size buffers.
// The helpers below replicate the exact per-component semantics of that chain
// so `convert_from_float` can build the target buffer in a single pass with
// byte-identical results.

/// `FloatImage::to_dynamic` quantization for 1/2-channel sources (truncating).
#[inline]
fn q16(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 65535.0) as u16
}

/// image crate `normalize_float` + rounding: NaN/+inf map to max, negatives to 0.
#[inline]
fn norm_f32(v: f32, max: f32) -> f32 {
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let clamped = if !(v < 1.0) { 1.0 } else { v.max(0.0) };
    (clamped * max).round()
}

/// image crate `FromPrimitive<f32> for u8`.
#[inline]
fn f32_to_u8(v: f32) -> u8 {
    norm_f32(v, u8::MAX as f32) as u8
}

/// image crate `FromPrimitive<f32> for u16`.
#[inline]
fn f32_to_u16(v: f32) -> u16 {
    norm_f32(v, u16::MAX as f32) as u16
}

/// image crate `FromPrimitive<u16> for u8` (round(c * 255 / 65535)); used on
/// the gray→gray fast path.
#[inline]
fn u16_to_u8(c: u16) -> u8 {
    ((c as u32 + 128) / 257) as u8
}

/// image crate `ColorComponentForCicp::expand_to_f32` for u16. Gray→RGB
/// conversions round-trip components through f32 with these exact semantics
/// (reciprocal multiply, no clamp).
#[inline]
fn expand16(c: u16) -> f32 {
    c as f32 * (1.0 / u16::MAX as f32)
}

/// image crate `ColorComponentForCicp::clamp_from_f32` for u8 (`as` saturates).
#[inline]
fn clamp_to_u8(v: f32) -> u8 {
    (v * u8::MAX as f32).round() as u8
}

/// image crate `ColorComponentForCicp::clamp_from_f32` for u16.
#[inline]
fn clamp_to_u16(v: f32) -> u16 {
    (v * u16::MAX as f32).round() as u16
}

/// Convert a `FloatImage` directly to the `DynamicImage` variant matching
/// the requested `ColorFormat`, in a single pass over the pixel data.
///
/// Byte-identical to the previous `to_dynamic()` + whole-buffer conversion
/// chain: 1/2-channel sources behave as if they had round-tripped through
/// Luma16/LumaA16, 3/4-channel sources as through Rgb32F/Rgba32F.
pub(crate) fn convert_from_float(data: &FloatImage, format: &ColorFormat) -> DynamicImage {
    let (w, h) = data.dimensions();
    let ch = data.channels() as usize;
    let src = data.as_raw();
    let px = src.chunks_exact(ch);

    match format {
        // RGB(A) sources going to a gray layout use the image crate's own
        // conversion (via `to_dynamic`, a raw clone for 3/4ch): its CICP
        // luminance coefficients are derived at runtime and cannot be
        // replicated here byte-exactly. Gray sources stay single-pass.
        ColorFormat::Gray8 => match ch {
            1 | 2 => {
                let out: Vec<u8> = px.map(|p| u16_to_u8(q16(p[0]))).collect();
                DynamicImage::ImageLuma8(ImageBuffer::from_raw(w, h, out).unwrap())
            }
            _ => DynamicImage::ImageLuma8(data.to_dynamic().to_luma8()),
        },
        ColorFormat::Gray16 => match ch {
            1 | 2 => {
                let out: Vec<u16> = px.map(|p| q16(p[0])).collect();
                DynamicImage::ImageLuma16(ImageBuffer::from_raw(w, h, out).unwrap())
            }
            _ => DynamicImage::ImageLuma16(data.to_dynamic().to_luma16()),
        },
        ColorFormat::GrayA8 => match ch {
            1 => {
                let out: Vec<u8> = px.flat_map(|p| [u16_to_u8(q16(p[0])), u8::MAX]).collect();
                DynamicImage::ImageLumaA8(ImageBuffer::from_raw(w, h, out).unwrap())
            }
            2 => {
                let out: Vec<u8> = px.flat_map(|p| [u16_to_u8(q16(p[0])), u16_to_u8(q16(p[1]))]).collect();
                DynamicImage::ImageLumaA8(ImageBuffer::from_raw(w, h, out).unwrap())
            }
            _ => DynamicImage::ImageLumaA8(data.to_dynamic().to_luma_alpha8()),
        },
        ColorFormat::GrayA16 => match ch {
            1 => {
                let out: Vec<u16> = px.flat_map(|p| [q16(p[0]), u16::MAX]).collect();
                DynamicImage::ImageLumaA16(ImageBuffer::from_raw(w, h, out).unwrap())
            }
            2 => {
                let out: Vec<u16> = px.flat_map(|p| [q16(p[0]), q16(p[1])]).collect();
                DynamicImage::ImageLumaA16(ImageBuffer::from_raw(w, h, out).unwrap())
            }
            _ => DynamicImage::ImageLumaA16(data.to_dynamic().to_luma_alpha16()),
        },
        ColorFormat::Rgb8 => {
            let out: Vec<u8> = match ch {
                1 | 2 => px.flat_map(|p| { let v = clamp_to_u8(expand16(q16(p[0]))); [v, v, v] }).collect(),
                _ => px.flat_map(|p| [f32_to_u8(p[0]), f32_to_u8(p[1]), f32_to_u8(p[2])]).collect(),
            };
            DynamicImage::ImageRgb8(ImageBuffer::from_raw(w, h, out).unwrap())
        }
        ColorFormat::Rgb16 => {
            let out: Vec<u16> = match ch {
                1 | 2 => px.flat_map(|p| { let v = clamp_to_u16(expand16(q16(p[0]))); [v, v, v] }).collect(),
                _ => px.flat_map(|p| [f32_to_u16(p[0]), f32_to_u16(p[1]), f32_to_u16(p[2])]).collect(),
            };
            DynamicImage::ImageRgb16(ImageBuffer::from_raw(w, h, out).unwrap())
        }
        ColorFormat::Rgb32F => {
            let out: Vec<f32> = match ch {
                1 | 2 => px.flat_map(|p| { let v = expand16(q16(p[0])); [v, v, v] }).collect(),
                3 => src.to_vec(),
                _ => px.flat_map(|p| [p[0], p[1], p[2]]).collect(),
            };
            DynamicImage::ImageRgb32F(ImageBuffer::from_raw(w, h, out).unwrap())
        }
        ColorFormat::Rgba8 => {
            let out: Vec<u8> = match ch {
                1 => px.flat_map(|p| { let v = clamp_to_u8(expand16(q16(p[0]))); [v, v, v, u8::MAX] }).collect(),
                2 => px.flat_map(|p| { let v = clamp_to_u8(expand16(q16(p[0]))); [v, v, v, clamp_to_u8(expand16(q16(p[1])))] }).collect(),
                3 => px.flat_map(|p| [f32_to_u8(p[0]), f32_to_u8(p[1]), f32_to_u8(p[2]), u8::MAX]).collect(),
                _ => px.flat_map(|p| [f32_to_u8(p[0]), f32_to_u8(p[1]), f32_to_u8(p[2]), f32_to_u8(p[3])]).collect(),
            };
            DynamicImage::ImageRgba8(ImageBuffer::from_raw(w, h, out).unwrap())
        }
        ColorFormat::Rgba16 => {
            let out: Vec<u16> = match ch {
                1 => px.flat_map(|p| { let v = clamp_to_u16(expand16(q16(p[0]))); [v, v, v, u16::MAX] }).collect(),
                2 => px.flat_map(|p| { let v = clamp_to_u16(expand16(q16(p[0]))); [v, v, v, clamp_to_u16(expand16(q16(p[1])))] }).collect(),
                3 => px.flat_map(|p| [f32_to_u16(p[0]), f32_to_u16(p[1]), f32_to_u16(p[2]), u16::MAX]).collect(),
                _ => px.flat_map(|p| [f32_to_u16(p[0]), f32_to_u16(p[1]), f32_to_u16(p[2]), f32_to_u16(p[3])]).collect(),
            };
            DynamicImage::ImageRgba16(ImageBuffer::from_raw(w, h, out).unwrap())
        }
        ColorFormat::Rgba32F => {
            let out: Vec<f32> = match ch {
                1 => px.flat_map(|p| { let v = expand16(q16(p[0])); [v, v, v, 1.0] }).collect(),
                2 => px.flat_map(|p| { let v = expand16(q16(p[0])); [v, v, v, expand16(q16(p[1]))] }).collect(),
                3 => px.flat_map(|p| [p[0], p[1], p[2], 1.0]).collect(),
                _ => src.to_vec(),
            };
            DynamicImage::ImageRgba32F(ImageBuffer::from_raw(w, h, out).unwrap())
        }
    }
}

/// Check whether the given color format is compatible with the image file format.
/// Returns `Ok(())` if compatible, or `Err(message)` describing why not.
pub(crate) fn check_compatibility(image_format: &ImageFormat, color_format: &ColorFormat) -> Result<(), String> {
    if color_format.is_compatible_with_image_format(image_format) {
        Ok(())
    } else {
        Err(format!(
            "{:?} does not support the {:?} color format.",
            image_format, color_format
        ))
    }
}

/// Parse the "png compression" dropdown text into a `PngCompression` value.
/// Returns `Err(message)` for any string other than fast/default/best/uncompressed
/// (case-insensitive, trimmed).
pub(crate) fn parse_png_compression(text: &str) -> Result<PngCompression, String> {
    match text.trim().to_lowercase().as_str() {
        "fast" => Ok(PngCompression::Fast),
        "default" => Ok(PngCompression::Default),
        "best" => Ok(PngCompression::Best),
        "uncompressed" => Ok(PngCompression::Uncompressed),
        other => Err(format!(
            "Unknown png compression '{}'; expected fast, default, best, or uncompressed.",
            other
        )),
    }
}

/// Map a save-path's file extension to the `ImageFormat` it selects.
///
/// Case-insensitively matches the path's extension against every supported
/// [`crate::value::ImageType`] (the same 14 formats enumerated by
/// `ImageType::types()`, which includes AVIF — write-only in this app, unlike
/// the read-side extension list used by the image "from file" input node).
/// Returns `Err` with a message listing the supported extensions if the path
/// has no extension or an unrecognized one.
pub fn image_format_from_path(path: &std::path::Path) -> Result<ImageFormat, String> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Err(format!(
            "File path has no extension; cannot determine image format. Supported extensions: {}.",
            supported_extensions_list()
        ));
    };
    let ext_lower = ext.to_lowercase();
    for image_type in crate::value::ImageType::types().iter() {
        if image_type.format().extensions_str().contains(&ext_lower.as_str()) {
            return Ok(image_type.format());
        }
    }
    Err(format!(
        "Unsupported image extension '{}'. Supported extensions: {}.",
        ext, supported_extensions_list()
    ))
}

/// Comma-separated list of the first (canonical) extension for every
/// supported image format, for error messages.
fn supported_extensions_list() -> String {
    crate::value::ImageType::types()
        .iter()
        .map(|t| t.format().extensions_str()[0])
        .collect::<Vec<_>>()
        .join(", ")
}

/// Convert a `FloatImage` to the requested `ColorFormat` and encode/save it to
/// `path` in `image_format`. JPEG/PNG/AVIF use explicit encoders so `quality`
/// (JPEG/AVIF) and `png_compression` (PNG) apply; other formats are saved via
/// `DynamicImage::save_with_format`, which has no encoder settings.
///
/// `color_format` must be compatible with `image_format` (see
/// [`ColorFormat::is_compatible_with_image_format`] /
/// [`ColorFormat::default_for_image_format`]) — this function does not
/// validate compatibility itself, matching the "to file" node which checks
/// before calling in.
pub fn save_image(
    path: &std::path::Path,
    data: &FloatImage,
    color_format: &ColorFormat,
    image_format: ImageFormat,
    quality: u8,
    png_compression: PngCompression,
) -> Result<(), String> {
    // Convert the FloatImage directly into the requested color format
    // in a single pass (no intermediate to_dynamic buffer).
    let converted = convert_from_float(data, color_format);

    match image_format {
        // JPEG, PNG, and AVIF use explicit encoders so quality/compression apply.
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Avif => std::fs::File::create(path)
            .map_err(|e| e.to_string())
            .and_then(|f| {
                let mut writer = BufWriter::new(f);
                match image_format {
                    ImageFormat::Jpeg => converted.write_with_encoder(JpegEncoder::new_with_quality(&mut writer, quality)),
                    // Speed 4 is the encoder's own default (cavif's choice).
                    ImageFormat::Avif => converted.write_with_encoder(AvifEncoder::new_with_speed_quality(&mut writer, 4, quality)),
                    _ => converted.write_with_encoder(PngEncoder::new_with_quality(&mut writer, png_compression, PngFilter::Adaptive)),
                }
                .map_err(|e| e.to_string())?;
                writer.flush().map_err(|e| e.to_string())
            }),
        // All other formats have no encoder settings: save directly. BMP/PNM
        // only pass validation as Rgb8/Gray8, which are already alpha-free;
        // HDR only as Rgb32F, matching its RGBE encoder.
        _ => converted.save_with_format(path, image_format).map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
