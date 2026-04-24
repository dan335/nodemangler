//! Image-to-file output operation.
//!
//! Saves an image to a file on disk, using a configurable file name, folder
//! path, image format (e.g., JPEG, PNG), and color format (e.g., Rgba8, Rgba16).
//! The input `FloatImage` is converted to a `DynamicImage` via [`FloatImage::to_dynamic`]
//! before encoding, so all existing format/codec logic works unchanged.
//! Outputs the resulting file path.

use image::ImageFormat;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType, ColorFormat};
use serde::{Deserialize, Serialize};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

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

    /// Convert a `DynamicImage` to the `DynamicImage` variant matching the
    /// requested `ColorFormat`.
    fn convert_to_format(img: &DynamicImage, format: &ColorFormat) -> DynamicImage {
        match format {
            ColorFormat::Rgba32F => DynamicImage::ImageRgba32F(img.to_rgba32f()),
            ColorFormat::Rgb32F => DynamicImage::ImageRgb32F(img.to_rgb32f()),
            ColorFormat::Rgba16 => DynamicImage::ImageRgba16(img.to_rgba16()),
            ColorFormat::Rgb16 => DynamicImage::ImageRgb16(img.to_rgb16()),
            ColorFormat::GrayA16 => DynamicImage::ImageLumaA16(img.to_luma_alpha16()),
            ColorFormat::Gray16 => DynamicImage::ImageLuma16(img.to_luma16()),
            ColorFormat::Rgba8 => DynamicImage::ImageRgba8(img.to_rgba8()),
            ColorFormat::Rgb8 => DynamicImage::ImageRgb8(img.to_rgb8()),
            ColorFormat::GrayA8 => DynamicImage::ImageLumaA8(img.to_luma_alpha8()),
            ColorFormat::Gray8 => DynamicImage::ImageLuma8(img.to_luma8()),
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

            // Convert FloatImage to DynamicImage for file encoding, then apply color format
            let dynamic = data.to_dynamic();
            let converted = Arc::new(Self::convert_to_format(&dynamic, &color_format));

            let save_result = match image_type {
                // JPEG uses a custom encoder for quality control
                ImageFormat::Jpeg => {
                    let file = std::fs::File::create(&folder_path);
                    match file {
                        Ok(f) => {
                            let writer = BufWriter::new(f);
                            let encoder = JpegEncoder::new_with_quality(writer, quality);
                            // JPEG only accepts Rgb8 or Gray8 (validated above)
                            match &color_format {
                                ColorFormat::Gray8 => {
                                    let gray = converted.to_luma8();
                                    encoder.write_image(
                                        gray.as_raw(),
                                        gray.width(),
                                        gray.height(),
                                        image::ExtendedColorType::L8,
                                    ).map_err(|e| e.to_string())
                                }
                                _ => {
                                    let rgb = converted.to_rgb8();
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
                // BMP/PNM: ensure no alpha (validated above, but strip just in case)
                ImageFormat::Bmp | ImageFormat::Pnm => {
                    let no_alpha = match &color_format {
                        ColorFormat::Gray8 => Arc::new(DynamicImage::ImageLuma8(converted.to_luma8())),
                        _ => Arc::new(DynamicImage::ImageRgb8(converted.to_rgb8())),
                    };
                    no_alpha.save_with_format(&folder_path, image_type).map_err(|e| e.to_string())
                }
                // All other formats: save the converted image directly
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
