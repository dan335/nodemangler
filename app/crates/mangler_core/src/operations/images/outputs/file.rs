//! Image-to-file output operation.
//!
//! Saves an image to a file on disk. The destination is composed from three
//! inputs — a `folder` (relative to where the graph is saved, unless absolute),
//! a `file name` (defaulting to the graph's name), and a `format` dropdown that
//! chooses the image format and hence the file extension. A color format (e.g.
//! Rgba8, Rgba16), JPEG/AVIF quality, and PNG compression level round out the
//! encoder settings.
//! The input `FloatImage` is converted directly into the `DynamicImage` variant
//! matching the requested color format in a single pass (see
//! [`super::convert_from_float`]), then handed to the encoder.
//! Writing is gated by the shared auto-save / manual-save mechanism (see
//! [`super::should_save_and_consume`]); the resulting file path is output.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType, ColorFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use super::{check_compatibility, parse_png_compression, save_image, save_gate_inputs, should_save_and_consume};
// `convert_from_float` is not called directly here (see `save_image`); this
// cfg(test)-only import lets `file_tests.rs`'s `use super::*;` reach it
// unqualified without an unused-import warning in normal builds.
#[cfg(test)]
use super::convert_from_float;

/// Input indices (the layout is a positional contract with `run` and the tests).
const IMAGE: usize = 0;
const FOLDER: usize = 1;
const FILE_NAME: usize = 2;
const FORMAT: usize = 3;
const QUALITY: usize = 4;
const COLOR_FORMAT: usize = 5;
const PNG_COMPRESSION: usize = 6;
const AUTO_SAVE: usize = 7;
const SAVE: usize = 8;

/// Operation that saves an image to a file on disk.
///
/// Accepts an image, a folder, a file name, an image format, JPEG/AVIF quality,
/// a color format, a PNG compression level, and the auto-save / save-button
/// gating inputs. Outputs the full path of the saved file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputFile {}

impl OpImageOutputFile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to file".to_string(),
            description: "Saves an image to a file.".to_string(),
            help: "Writes the input image to `{folder}/{file name}.{format}`. The folder is resolved relative to where the graph is saved (so a graph and its outputs move together) unless you give an absolute path; leave it empty to write next to the graph file. The folder is created if it doesn't exist. The file name defaults to the graph's name when left blank. The format dropdown chooses the image format and the file extension — supported: png, jpg/jpeg, gif, webp, pnm, tiff, tga, bmp, ico, hdr, exr, ff (farbfeld), avif, qoi; it defaults to jpg.\n\nThe color format selector controls the output bit depth and channel layout (Gray8/16, GrayA8/16, Rgb8/16/32F, Rgba8/16/32F). The quality slider applies to the lossy formats (JPEG and AVIF), and the png compression selector only to PNG (all PNG settings produce identical pixels — only file size and encode time differ). WebP is always encoded losslessly; Radiance HDR writes from the Rgb32F color format; the remaining formats have no encoder settings. An incompatible format/color-format combination (for example an RGBA channel layout into a JPEG) is rejected before any file is written, and the full saved path is returned as an output for chaining.\n\nWriting is off by default: turn on auto save to write whenever an input changes, or leave it off and press the save button to write once. Headless `mangle run` always writes regardless of the toggle.".to_string(),
        }
    }

    /// Creates the input definitions. See the index constants for the order,
    /// which is a positional contract with [`Self::run`].
    pub fn create_inputs() -> Vec<Input> {
        let mut inputs = vec![
            Input::new("image".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Image to encode and save to disk."),
            Input::new("folder".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: Some("output folder".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFolder,
            }), None)
                .with_description("Destination folder (absolute, or relative to where the graph is saved). Pre-filled with the graph's own folder when the node is created; empty = the graph's own folder."),
            Input::new("file name".to_string(), Value::Text(String::new()), Some(InputSettings::SingleLineText), None)
                .with_description("Output file name (without extension). Empty = the graph's name."),
            Input::new("format".to_string(), Value::ImageType(image::ImageFormat::Jpeg), None, None)
                .with_description("Image format; also selects the file extension."),
            Input::new("quality".to_string(), Value::Integer(85), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Lossy compression quality from 1 (smallest) to 100 (best); applies to JPEG and AVIF."),
            Input::new("color format".to_string(), Value::ColorFormat(ColorFormat::Rgb8), None, None)
                .with_description("Pixel encoding (bit depth and channel layout) used to write the file."),
            Input::new("png compression".to_string(), Value::Text("fast".to_string()), Some(InputSettings::Dropdown {
                options: vec!["fast".to_string(), "default".to_string(), "best".to_string(), "uncompressed".to_string()],
            }), None)
                .with_description("PNG compression effort (lossless; affects file size and encode time only). Ignored for other formats."),
        ];
        inputs.extend(save_gate_inputs());
        inputs
    }

    /// Creates the output definitions: the full file path where the image was saved.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("file path".to_string(), Value::Path(PathBuf::new()), None)
                .with_description("Full path of the file that was written (empty when nothing was written)."),
        ]
    }

    /// Executes the operation: saves the image to disk at the resolved path.
    ///
    /// When auto save is off and the save button hasn't been pressed (and the
    /// engine isn't forcing saves), this is a no-op that returns an empty path.
    /// Otherwise it resolves the folder/name/format, creates the folder, and
    /// writes the file. Returns an error if the file name is empty, the color
    /// format is incompatible with the image format, the folder cannot be
    /// created, or the image cannot be encoded.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        // Decide up front whether to write, consuming the one-shot save pulse
        // (mutably borrows `inputs`, so it must precede the immutable
        // conversions below).
        let should_save = should_save_and_consume(inputs, AUTO_SAVE, SAVE);

        let mut input_errors: Vec<(usize, String)> = vec![];
        let image_converted = convert_input(inputs, IMAGE, ValueType::Image, &mut input_errors);
        let folder_converted = convert_input(inputs, FOLDER, ValueType::Path, &mut input_errors);
        let name_converted = convert_input(inputs, FILE_NAME, ValueType::Text, &mut input_errors);
        let format_converted = convert_input(inputs, FORMAT, ValueType::ImageType, &mut input_errors);
        let quality_converted = convert_input(inputs, QUALITY, ValueType::Integer, &mut input_errors);
        let color_format_converted = convert_input(inputs, COLOR_FORMAT, ValueType::ColorFormat, &mut input_errors);
        let png_compression_converted = convert_input(inputs, PNG_COMPRESSION, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Nothing to write this run: report an empty path and skip all
        // validation so an idle manual-mode node never shows an error.
        if !should_save {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Path(PathBuf::new()) }],
            });
        }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Path(folder) = folder_converted.unwrap() else { unreachable!() };
        let Value::Text(file_name) = name_converted.unwrap() else { unreachable!() };
        let Value::ImageType(image_type) = format_converted.unwrap() else { unreachable!() };
        let Value::Integer(quality) = quality_converted.unwrap() else { unreachable!() };
        let quality = quality.clamp(1, 100) as u8;
        let Value::ColorFormat(color_format) = color_format_converted.unwrap() else { unreachable!() };
        let Value::Text(png_compression_text) = png_compression_converted.unwrap() else { unreachable!() };

        // Resolve the destination folder and file stem from the graph context
        // (shared with the `material` node so both behave identically).
        let (resolved_dir, stem) = super::resolve_output_dir_and_stem(&folder, &file_name, FOLDER, FILE_NAME)?;

        // Validate the color format against the chosen image format before
        // touching the filesystem.
        if let Err(msg) = check_compatibility(&image_type, &color_format) {
            return Err(OperationError { input_errors: vec![(COLOR_FORMAT, msg.clone())], node_error: Some(msg) });
        }

        let png_compression = match parse_png_compression(&png_compression_text) {
            Ok(v) => v,
            Err(msg) => {
                return Err(OperationError { input_errors: vec![(PNG_COMPRESSION, msg.clone())], node_error: Some(msg) });
            }
        };

        // Create the destination folder if needed (relative-to-graph folders are
        // meant to be authored freely), then build the full path.
        if let Err(e) = std::fs::create_dir_all(&resolved_dir) {
            let msg = format!("Could not create folder '{}': {}", resolved_dir.display(), e);
            return Err(OperationError { input_errors: vec![(FOLDER, msg.clone())], node_error: Some(msg) });
        }
        let ext = image_type.extensions_str()[0];
        let path = resolved_dir.join(format!("{}.{}", stem, ext));

        match save_image(&path, &data, &color_format, image_type, quality, png_compression) {
            Ok(_) => Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse {
                    value: Value::Path(path),
                }],
            }),
            Err(e) => Err(OperationError { input_errors: vec![], node_error: Some(format!("Failed to save image: {}", e)) }),
        }
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
