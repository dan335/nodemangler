//! Image-from-file input operation.
//!
//! Reads an image from a local file path and outputs the decoded image
//! along with its width and height. The loaded `DynamicImage` is converted
//! to a `FloatImage` via [`FloatImage::from_dynamic`], preserving the
//! original channel count (grayscale stays 1ch, RGB 3ch, etc.).

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
            help: "Decodes an image file from disk using the image crate and converts it into a FloatImage, preserving the source channel count (grayscale stays 1ch, RGB 3ch, RGBA 4ch). The path input uses a picker filtered to the supported image extensions.\n\nErrors if the file cannot be opened or the format is unsupported. Note that pixel values are interpreted as sRGB by default; connect a linear-RGB conversion downstream if the file holds linear data like a normal or height map.".to_string(),
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

        // run node
        let mut width = 0;
        let mut height = 0;
        let mut img = None;

        if let Ok(open) = ImageReader::open(path) {
            if let Ok(dynamic_image) = open.decode() {
                // Convert to FloatImage, preserving original channel count
                let float_img = FloatImage::from_dynamic(&dynamic_image);
                width = float_img.width();
                height = float_img.height();
                img = Some(float_img);
            }
        }

        if let Some(value) = img {
            Ok(OperationResponse { 
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::new(value), change_id: get_id() } },
                    OutputResponse { value: Value::Integer(width as i32) },
                    OutputResponse { value: Value::Integer(height as i32) },
                ],
            })
        } else {
            Err(OperationError { input_errors, node_error: Some("Error opening image.".to_string()) })
        }
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
