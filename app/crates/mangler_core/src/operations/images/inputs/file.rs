//! Image-from-file input operation.
//!
//! Reads an image from a local file path and outputs the decoded image
//! along with its width and height. Most formats decode through the image
//! crate into a `DynamicImage`, converted to a `FloatImage` via
//! [`FloatImage::from_dynamic`], preserving the original channel count
//! (grayscale stays 1ch, RGB 3ch, etc.). JPEG XL (via jxl-oxide) and PSD
//! (via psd, flattened composite) are decoded by dedicated pure-Rust crates.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use image::ImageReader;

/// Operation that loads an image from a file on disk.
///
/// Accepts a file path input with an extension filter matching supported image
/// formats, and produces the decoded image plus its dimensions as outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputFile {}

impl OpImageInputFile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from file".to_string(),
            description: "Grabs an image from a file.".to_string(),
            help: "Decodes an image file from disk and converts it into a FloatImage, preserving the source channel count (grayscale stays 1ch, RGB 3ch, RGBA 4ch). The path input uses a picker filtered to the supported image extensions. JPEG XL files are decoded with jxl-oxide and PSD files with the psd crate (the flattened composite image; individual layers are not exposed).\n\nErrors if the file cannot be opened or the format is unsupported. Note that pixel values are interpreted as sRGB by default; connect a linear-RGB conversion downstream if the file holds linear data like a normal or height map.".to_string(),
        }
    }

    /// Creates the input definitions: a single file path input with image extension filtering.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("path".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path{
                extension_filter: ValueType::file_extensions(&ValueType::Image),
                set_directory: None,
                set_file_name: None,
                set_title: Some("image".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }), None)
                .with_description("Path to an image file to load from disk."),
        ]
    }

    /// Creates the output definitions: the decoded image, its width, and its height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Image decoded from the file on disk."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Width of the loaded image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Height of the loaded image in pixels."),
        ]
    }

    /// Executes the operation: reads and decodes the image file at the given path.
    ///
    /// Returns an error if the file cannot be opened or the image format is unsupported.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let path_converted = convert_input(inputs, 0, ValueType::Path, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Path(path) = path_converted.unwrap() else { unreachable!() };

        // run node — JPEG XL and PSD have dedicated decoders; everything else
        // goes through the image crate.
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let decode_result = match extension.as_deref() {
            Some("jxl") => Self::decode_jxl(&path),
            Some("psd") => Self::decode_psd(&path),
            _ => ImageReader::open(&path)
                .map_err(|e| e.to_string())
                .and_then(|reader| reader.decode().map_err(|e| e.to_string()))
                .map(|dynamic_image| FloatImage::from_dynamic(&dynamic_image)),
        };

        match decode_result {
            Ok(float_img) => {
                let width = float_img.width();
                let height = float_img.height();
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![
                        OutputResponse { value: Value::Image { data: Arc::new(float_img), change_id: get_id() } },
                        OutputResponse { value: Value::Integer(width as i32) },
                        OutputResponse { value: Value::Integer(height as i32) },
                    ],
                })
            }
            Err(e) => Err(OperationError { input_errors, node_error: Some(format!("Error opening image: {}", e)) }),
        }
    }

    /// Decodes a JPEG XL file with jxl-oxide (first frame for animations).
    ///
    /// The stream API yields interleaved f32 color + alpha channels, which map
    /// directly onto `FloatImage` semantics (1ch gray … 4ch RGBA).
    fn decode_jxl(path: &std::path::Path) -> Result<FloatImage, String> {
        let image = jxl_oxide::JxlImage::open_with_defaults(path).map_err(|e| e.to_string())?;
        let render = image.render_frame(0).map_err(|e| e.to_string())?;
        let mut stream = render.stream();
        let (width, height, channels) = (stream.width(), stream.height(), stream.channels());
        if channels == 0 || channels > 4 {
            return Err(format!("Unsupported JPEG XL channel count: {}", channels));
        }
        let mut buf = vec![0f32; width as usize * height as usize * channels as usize];
        stream.write_to_buffer(&mut buf);
        FloatImage::from_raw(width, height, channels, buf)
            .ok_or_else(|| "JPEG XL decode produced a mismatched buffer size.".to_string())
    }

    /// Decodes a PSD file with the psd crate.
    ///
    /// Uses the flattened composite image (individual layers are not exposed),
    /// which the crate always returns as 8-bit RGBA.
    fn decode_psd(path: &std::path::Path) -> Result<FloatImage, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let parsed = psd::Psd::from_bytes(&bytes).map_err(|e| e.to_string())?;
        let (width, height) = (parsed.width(), parsed.height());
        let data: Vec<f32> = parsed.rgba().iter().map(|&v| v as f32 / 255.0).collect();
        FloatImage::from_raw(width, height, 4, data)
            .ok_or_else(|| "PSD decode produced a mismatched buffer size.".to_string())
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
