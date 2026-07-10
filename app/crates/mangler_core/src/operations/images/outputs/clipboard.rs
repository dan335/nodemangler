//! Image-to-clipboard output operation.
//!
//! Copies an image to the system clipboard using the `arboard` crate,
//! making it available for pasting into other applications. The input
//! `FloatImage` is converted to RGBA8 via [`FloatImage::to_rgba8`] before
//! writing to the clipboard.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use arboard::{Clipboard, ImageData};

use super::{save_gate_inputs, should_save_and_consume};

/// Input indices (positional contract with `run`).
const IMAGE: usize = 0;
const AUTO_SAVE: usize = 1;
const SAVE: usize = 2;

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
            name: "to clipboard".to_string(),
            description: "Copies an image to the clipboard.".to_string(),
            help: "Converts the input FloatImage to RGBA8 and writes it to the system clipboard via arboard so it can be pasted into other applications. Quantisation to 8 bits happens at this step, so high-dynamic-range or linear data will be clipped and gamma-encoded.\n\nThis node has no output sockets; its effect is purely the clipboard side effect. It will error if the platform clipboard is unavailable or rejects the image.\n\nCopying is off by default: turn on auto save to copy whenever the input changes, or leave it off and press the save button to copy once. Headless `mangle run` always copies regardless of the toggle.".to_string(),
        }
    }

    /// Creates the input definitions: an image plus the auto-save / save-button
    /// gating inputs.
    pub fn create_inputs() -> Vec<Input> {
        let mut inputs = vec![
            Input::new("image".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Image to write to the system clipboard as RGBA8."),
        ];
        inputs.extend(save_gate_inputs());
        inputs
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

        // Decide whether to copy this run, consuming the one-shot save pulse
        // (mutable borrow — must precede the conversions below).
        let should_copy = should_save_and_consume(inputs, AUTO_SAVE, SAVE);

        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, IMAGE, ValueType::Image, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Nothing to copy this run (auto save off, button not pressed, not forced).
        if !should_copy {
            return Ok(OperationResponse { time: Instant::now().duration_since(start_time), responses: vec![] });
        }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node — convert FloatImage to RGBA8 and prepare arboard ImageData
        let rgba8 = data.to_rgba8();
        let image_data = ImageData {
            width: data.width() as usize,
            height: data.height() as usize,
            bytes: std::borrow::Cow::Borrowed(rgba8.as_flat_samples().samples)
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
#[path = "clipboard_tests.rs"]
mod tests;
