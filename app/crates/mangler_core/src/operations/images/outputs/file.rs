//! Image-to-file output operation.
//!
//! Saves an image to a file on disk, using a configurable file name, folder
//! path, image format (e.g., JPEG, PNG), color format (e.g., Rgba8, Rgba16),
//! JPEG quality, and PNG compression level.
//! The input `FloatImage` is converted directly into the `DynamicImage` variant
//! matching the requested color format in a single pass (see
//! [`super::convert_from_float`]), then handed to the encoder.
//! Outputs the resulting file path.

use image::ImageFormat;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType, ColorFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use super::{check_compatibility, parse_png_compression, save_image};
// `convert_from_float` is not called directly here (see `save_image`); this
// cfg(test)-only import lets `file_tests.rs`'s `use super::*;` reach it
// unqualified without an unused-import warning in normal builds.
#[cfg(test)]
use super::convert_from_float;

/// Operation that saves an image to a file on disk.
///
/// Accepts an image, a file name (without extension), a folder path, an
/// image format, JPEG quality, a color format, and a PNG compression level.
/// The extension is derived from the chosen format. Outputs the full path of
/// the saved file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputFile {}

impl OpImageOutputFile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to file".to_string(),
            description: "Saves an image to a file.".to_string(),
            help: "Encodes the input image and writes it into the chosen folder under the given base filename; the extension is appended automatically based on the selected image format. The color format selector controls the output bit depth and channel layout (Gray8/16, GrayA8/16, Rgb8/16/32F, Rgba8/16/32F).\n\nThe quality slider applies to the lossy formats (JPEG and AVIF), and the png compression selector only to PNG (all PNG settings produce identical pixels — only file size and encode time differ). WebP is always encoded losslessly; Radiance HDR writes from the Rgb32F color format; the remaining formats have no encoder settings. Incompatible format/color-format combinations (for example an RGBA channel layout into a JPEG) are rejected before any file is written, and the full saved path is returned as an output for chaining.".to_string(),
        }
    }

    /// Creates the input definitions: image, file name, folder path, image format,
    /// JPEG quality, color format, and PNG compression.
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
            Input::new("quality".to_string(), Value::Integer(85), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Lossy compression quality from 1 (smallest) to 100 (best); applies to JPEG and AVIF."),
            Input::new("color format".to_string(), Value::ColorFormat(ColorFormat::Rgb8), None, None)
                .with_description("Pixel encoding (bit depth and channel layout) used to write the file."),
            Input::new("png compression".to_string(), Value::Text("fast".to_string()), Some(InputSettings::Dropdown {
                options: vec!["fast".to_string(), "default".to_string(), "best".to_string(), "uncompressed".to_string()],
            }), None)
                .with_description("PNG compression effort (lossless; affects file size and encode time only). Ignored for other formats."),
        ]
    }

    /// Creates the output definitions: the full file path where the image was saved.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("file path".to_string(), Value::Path(PathBuf::new()), None)
                .with_description("Full path of the file that was written."),
        ]
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
        // PNG compression input (index 6) — default to "fast" (the encoder's own
        // default, matching the previous save behaviour) if missing (backwards compat)
        let png_compression_converted = if inputs.len() > 6 {
            convert_input(inputs, 6, ValueType::Text, &mut input_errors)
        } else {
            Some(Value::Text("fast".to_string()))
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
        let Value::Text(png_compression_text) = png_compression_converted.unwrap() else { unreachable!() };

        let png_compression = match parse_png_compression(&png_compression_text) {
            Ok(v) => v,
            Err(msg) => {
                return Err(OperationError { input_errors: vec![(6, msg.clone())], node_error: Some(msg) });
            }
        };

        if file_name.trim().is_empty() {
            let msg = "File name is empty.".to_string();
            return Err(OperationError { input_errors: vec![(1, msg.clone())], node_error: Some(msg) });
        }

        // Validate that the color format is compatible with the image format
        if let Err(msg) = check_compatibility(&image_type, &color_format) {
            return Err(OperationError {
                input_errors: vec![(5, msg.clone())],
                node_error: Some(msg),
            });
        }

        // run node — build the full output path from folder + file name + format extension
        if !folder_path.is_dir() {
            return Err(OperationError { input_errors: vec![], node_error: Some("Folder does not exist or is not a directory.".to_string()) });
        }
        // Append the extension rather than using `set_extension`, which would
        // eat everything after a dot in the file name ("render.v2" → "render.png").
        folder_path.push(format!("{}.{}", file_name, image_type.extensions_str()[0]));

        match save_image(&folder_path, &data, &color_format, image_type, quality, png_compression) {
            Ok(_) => Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse {
                    value: Value::Path(folder_path),
                }],
            }),
            Err(e) => Err(OperationError { input_errors: vec![], node_error: Some(format!("Failed to save image: {}", e)) }),
        }
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
