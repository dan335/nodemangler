//! Image-to-clipboard output operation.
//!
//! Copies an image to the system clipboard using the `arboard` crate,
//! making it available for pasting into other applications.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use arboard::{Clipboard, ImageData};

/// Operation that copies an image to the system clipboard.
///
/// Converts the input image to RGBA8 format and writes it to the clipboard
/// via `arboard`. This operation has no outputs since the result is a
/// side effect (clipboard contents).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageOutputClipboard {}

impl OpImageOutputClipboard {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image to clipboard".to_string(),
            description: "Copies an image to the clipboard.".to_string(),
        }
    }

    /// Creates the input definitions: a single image to copy to the clipboard.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the output definitions: none (clipboard write is a side effect).
    pub fn create_outputs() -> Vec<Output> {
        vec![]
    }

    /// Executes the operation: converts the image to RGBA8 and writes it to the clipboard.
    ///
    /// Returns an error if the clipboard cannot be accessed or the image cannot be written.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node — convert to RGBA8 and prepare arboard ImageData
        let rgba8 = data.to_rgba8();
        let image_data = ImageData {
            width: data.width() as usize,
            height: data.height() as usize,
            bytes: std::borrow::Cow::Borrowed( rgba8.as_flat_samples().samples)
        };
        
        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_image(image_data).is_ok() {
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![],
                })
            } else {
                Err(OperationError { input_errors: vec![], node_error: Some("Unable to copy image to clipboard.".to_string())  })
            }
        } else {
            Err(OperationError { input_errors: vec![], node_error: Some("Unable to convert to path.".to_string()) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_output_settings() {
        let s = OpImageOutputClipboard::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageOutputClipboard::create_inputs().is_empty());
        assert_eq!(OpImageOutputClipboard::create_outputs().len(), 0);
    }
}
