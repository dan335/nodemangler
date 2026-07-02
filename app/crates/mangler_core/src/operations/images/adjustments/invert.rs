//! Color inversion operation for images.
//!
//! Inverts each pixel's non-alpha channels so that `new = 1.0 - old`,
//! producing a photographic negative effect. Alpha is preserved.

use crate::get_id;
use crate::value::ValueType;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Color inversion operation that produces a photographic negative of the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentInvert {}

impl OpImageAdjustmentInvert {
    /// Returns the node metadata (name and description) for the invert operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "invert".to_string(),
            description: "Inverts all color channels of an image.".to_string(),
            help: "Replaces each colour channel with 1 minus its value, producing a photographic negative. Alpha is skipped so transparency is preserved.\n\nWhen the input image is the sole owner of its pixel data the node mutates in place instead of cloning, keeping it cheap in long chains. The operation assumes channels sit inside 0-1; values outside that interval are still inverted linearly but will remain outside 0-1, which downstream nodes may clip.".to_string(),
        }
    }

    /// Creates the input port: a single image to invert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to produce a photographic negative of."),
        ]
    }

    /// Creates the output port: the color-inverted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with each colour channel replaced by 1 minus its value; alpha preserved."),
        ]
    }

    /// Executes the invert operation. Attempts to unwrap the Arc to avoid cloning when possible.
    /// Inverts each non-alpha channel: `pixel[c] = 1.0 - pixel[c]`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node — try to take ownership of the image data to avoid cloning if possible
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        let ch = data_inner.channels() as usize;
        // Determine how many color channels to invert (skip alpha if present)
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in data_inner.pixels_mut() {
            for val in pixel.iter_mut().take(color_ch) {
                *val = 1.0 - *val;
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data: Arc::new(data_inner), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "invert_tests.rs"]
mod tests;
