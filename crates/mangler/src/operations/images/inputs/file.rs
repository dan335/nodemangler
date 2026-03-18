//! Image-from-file input operation.
//!
//! Reads an image from a local file path and outputs the decoded image
//! along with its width and height.

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
use image::io::Reader as ImageReader;

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
        }
    }

    /// Creates the input definitions: a single file path input with image extension filtering.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("path".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path{
                extension_filter: ValueType::file_extensions(&ValueType::DynamicImage),
                set_directory: None,
                set_file_name: None,
                set_title: Some("image".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }), None),
        ]
    }

    /// Creates the output definitions: the decoded image, its width, and its height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the operation: reads and decodes the image file at the given path.
    ///
    /// Returns an error if the file cannot be opened or the image format is unsupported.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let path_converted = convert_input(inputs, 0, ValueType::Path, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Path(path) = path_converted.unwrap() else { unreachable!() };

        // run node
        let mut width = 0;
        let mut height = 0;
        let mut img = None;

        if let Ok(open) = ImageReader::open(path) {
            if let Ok(dynamic_image) = open.decode() {
                width = dynamic_image.width();
                height = dynamic_image.height();
                img = Some(dynamic_image);
            }
        }

        if let Some(value) = img {
            Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::DynamicImage { data: Arc::new(value), change_id: get_id() } },
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_input_settings() {
        let s = OpImageInputFile::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageInputFile::create_inputs().is_empty());
        assert!(!OpImageInputFile::create_outputs().is_empty());
    }

    #[tokio::test]
    async fn test_file_input_exact_settings() {
        let s = OpImageInputFile::settings();
        assert_eq!(s.name, "from file");
        assert_eq!(OpImageInputFile::create_inputs().len(), 1);
        assert_eq!(OpImageInputFile::create_outputs().len(), 3);
    }

    #[tokio::test]
    async fn test_file_input_nonexistent_path_returns_error() {
        use crate::input::Input;
        let mut inputs = vec![
            Input::new("path".to_string(), Value::Path(PathBuf::from("/this/does/not/exist.png")), None, None),
        ];
        let result = OpImageInputFile::run(&mut inputs).await;
        assert!(result.is_err(), "loading from nonexistent path should fail");
    }
}
