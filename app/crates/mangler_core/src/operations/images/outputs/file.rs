//! Image-to-file output operation.
//!
//! Saves an image to a file on disk, using a configurable file name, folder
//! path, image format (e.g., JPEG, PNG), and color format (e.g., Rgba8, Rgba16).
//! The input `FloatImage` is converted directly into the `DynamicImage` variant
//! matching the requested color format in a single pass (see
//! [`OpImageOutputFile::convert_from_float`]), then handed to the encoder.
//! Outputs the resulting file path.

use image::ImageFormat;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageBuffer, ImageEncoder};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType, ColorFormat};
use serde::{Deserialize, Serialize};
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;

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

/// Operation that saves an image to a file on disk.
///
/// Accepts an image, a file name (without extension), a folder path, an
/// image format, JPEG quality, and a color format. The extension is derived
/// from the chosen format. Outputs the full path of the saved file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputFile {}

impl OpImageOutputFile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to file".to_string(),
            description: "Saves an image to a file.".to_string(),
            help: "Encodes the input image and writes it into the chosen folder under the given base filename; the extension is appended automatically based on the selected image format. The color format selector controls the output bit depth and channel layout (Gray8/16, GrayA8/16, Rgb8/16/32F, Rgba8/16/32F).\n\nThe jpg quality slider only applies when saving as JPEG. Incompatible format/color-format combinations (for example an RGBA channel layout into a JPEG) are rejected before any file is written, and the full saved path is returned as an output for chaining.".to_string(),
        }
    }

    /// Creates the input definitions: image, file name, folder path, image format,
    /// JPEG quality, and color format.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Image to encode and save to disk."),
            Input::new("file name".to_string(), Value::Text("image01".to_string()), Some(InputSettings::SingleLineText), None)
                .with_description("Base filename for the saved file; extension is appended automatically."),
            Input::new("folder".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: None,
                file_dialog_type: crate::input::FileDialogType::PickFolder,
            }), None)
                .with_description("Destination folder where the image file will be written."),
            Input::new("image format".to_string(), Value::ImageType(ImageFormat::Jpeg), None, None)
                .with_description("Image container format (JPEG, PNG, etc.) that determines the extension."),
            Input::new("jpg quality".to_string(), Value::Integer(85), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("JPEG compression quality from 1 (smallest) to 100 (best)."),
            Input::new("color format".to_string(), Value::ColorFormat(ColorFormat::Rgb8), None, None)
                .with_description("Pixel encoding (bit depth and channel layout) used to write the file."),
        ]
    }

    /// Creates the output definitions: the full file path where the image was saved.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("file path".to_string(), Value::Path(PathBuf::new()), None)
                .with_description("Full path of the file that was written."),
        ]
    }

    /// Convert a `FloatImage` directly to the `DynamicImage` variant matching
    /// the requested `ColorFormat`, in a single pass over the pixel data.
    ///
    /// Byte-identical to the previous `to_dynamic()` + whole-buffer conversion
    /// chain: 1/2-channel sources behave as if they had round-tripped through
    /// Luma16/LumaA16, 3/4-channel sources as through Rgb32F/Rgba32F.
    fn convert_from_float(data: &FloatImage, format: &ColorFormat) -> DynamicImage {
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
    fn check_compatibility(image_format: &ImageFormat, color_format: &ColorFormat) -> Result<(), String> {
        if color_format.is_compatible_with_image_format(image_format) {
            Ok(())
        } else {
            Err(format!(
                "{:?} does not support the {:?} color format.",
                image_format, color_format
            ))
        }
    }

    /// Executes the operation: saves the image to disk at the specified location.
    ///
    /// Returns an error if the folder does not exist, the color format is
    /// incompatible with the image format, or the image cannot be encoded.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let file_name_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let folder_converted = convert_input(inputs, 2, ValueType::Path, &mut input_errors);
        let image_type_converted = convert_input(inputs, 3, ValueType::ImageType, &mut input_errors);
        let quality_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        // Color format input (index 5) — default to Rgba8 if missing (backwards compat)
        let color_format = if inputs.len() > 5 {
            convert_input(inputs, 5, ValueType::ColorFormat, &mut input_errors)
        } else {
            Some(Value::ColorFormat(ColorFormat::Rgba8))
        };

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Text(file_name) = file_name_converted.unwrap() else { unreachable!() };
        let Value::Path(mut folder_path) = folder_converted.unwrap() else { unreachable!() };
        let Value::ImageType(image_type) = image_type_converted.unwrap() else { unreachable!() };
        let Value::Integer(quality) = quality_converted.unwrap() else { unreachable!() };
        let quality = quality.clamp(1, 100) as u8;
        let Value::ColorFormat(color_format) = color_format.unwrap() else { unreachable!() };

        // Validate that the color format is compatible with the image format
        if let Err(msg) = Self::check_compatibility(&image_type, &color_format) {
            return Err(OperationError {
                input_errors: vec![(5, msg.clone())],
                node_error: Some(msg),
            });
        }

        // run node — build the full output path from folder + file name + format extension
        if folder_path.exists() {
            folder_path.push(file_name);
            folder_path.set_extension(image_type.extensions_str()[0]);

            // Convert the FloatImage directly into the requested color format
            // in a single pass (no intermediate to_dynamic buffer).
            let converted = Self::convert_from_float(&data, &color_format);

            let save_result = match image_type {
                // JPEG uses a custom encoder for quality control
                ImageFormat::Jpeg => {
                    let file = std::fs::File::create(&folder_path);
                    match file {
                        Ok(f) => {
                            let writer = BufWriter::new(f);
                            let encoder = JpegEncoder::new_with_quality(writer, quality);
                            // JPEG only accepts Rgb8 or Gray8 (validated above),
                            // so `converted` is already in the right layout.
                            match converted {
                                DynamicImage::ImageLuma8(gray) => encoder.write_image(
                                    gray.as_raw(),
                                    gray.width(),
                                    gray.height(),
                                    image::ExtendedColorType::L8,
                                ).map_err(|e| e.to_string()),
                                other => {
                                    // Rgb8 by validation; `into_rgb8` is a move
                                    // for that variant, not a conversion.
                                    let rgb = other.into_rgb8();
                                    encoder.write_image(
                                        rgb.as_raw(),
                                        rgb.width(),
                                        rgb.height(),
                                        image::ExtendedColorType::Rgb8,
                                    ).map_err(|e| e.to_string())
                                }
                            }
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }
                // All other formats: save the converted image directly. BMP/PNM
                // only pass validation as Rgb8/Gray8, which are already alpha-free.
                _ => converted.save_with_format(&folder_path, image_type).map_err(|e| e.to_string()),
            };

            match save_result {
                Ok(_) => Ok(OperationResponse { 
                    time: Instant::now().duration_since(start_time),
                    responses: vec![OutputResponse {
                        value: Value::Path(folder_path),
                    }],
                }),
                Err(e) => Err(OperationError { input_errors: vec![], node_error: Some(format!("Failed to save image: {}", e)) }),
            }
        } else {
            Err(OperationError { input_errors: vec![], node_error: Some("Folder does not exist.".to_string()) })
        }


    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
