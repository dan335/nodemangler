//! Image-to-file output operation.
//!
//! Saves an image to a file on disk, using a configurable file name, folder
//! path, and image format (e.g., JPEG, PNG). Outputs the resulting file path.

use image::ImageFormat;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Operation that saves an image to a file on disk.
///
/// Accepts an image, a file name (without extension), a folder path, and an
/// image format. The extension is derived from the chosen format. Outputs the
/// full path of the saved file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputFile {}

impl OpImageOutputFile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image to file".to_string(),
            description: "Saves an image to a file.".to_string(),
        }
    }

    /// Creates the input definitions: image, file name, folder path, and image format.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("file name".to_string(), Value::Text("image01".to_string()), Some(InputSettings::SingleLineText), None),
            Input::new("folder".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: None,
                file_dialog_type: crate::input::FileDialogType::PickFolder,
            }), None),
            Input::new("image format".to_string(), Value::ImageType(ImageFormat::Jpeg), None, None),
        ]
    }

    /// Creates the output definitions: the full file path where the image was saved.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("file path".to_string(), Value::Path(PathBuf::new()), None),
        ]
    }

    /// Executes the operation: saves the image to disk at the specified location.
    ///
    /// Returns an error if the folder does not exist or the image cannot be encoded
    /// in the requested format.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let file_name_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let folder_converted = convert_input(inputs, 2, ValueType::Path, &mut input_errors);
        let image_type_converted = convert_input(inputs, 3, ValueType::ImageType, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Text(file_name) = file_name_converted.unwrap() else { unreachable!() };
        let Value::Path(mut folder_path) = folder_converted.unwrap() else { unreachable!() };
        let Value::ImageType(image_type) = image_type_converted.unwrap() else { unreachable!() };

        // run node — build the full output path from folder + file name + format extension
        if folder_path.exists() {
            folder_path.push(file_name);
            folder_path.set_extension(image_type.extensions_str()[0]);

            // Auto-convert RGBA to RGB for formats that don't support alpha (e.g. JPEG).
            let save_image = match image_type {
                ImageFormat::Jpeg | ImageFormat::Bmp | ImageFormat::Pnm => {
                    std::sync::Arc::new(image::DynamicImage::ImageRgb8(data.to_rgb8()))
                }
                _ => data,
            };

            if let Ok(_) = save_image.save_with_format(folder_path.clone(), image_type) {
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![OutputResponse {
                        value: Value::Path(folder_path),
                    }],
                })
            } else {
                Err(OperationError { input_errors: vec![], node_error: Some("Unable to convert to path.".to_string()) })
            }
        } else {
            Err(OperationError { input_errors: vec![], node_error: Some("Folder does not exist.".to_string()) })
        }

        
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
