//! Color inversion operation for images.
//!
//! Inverts each pixel's color channels (R, G, B) so that `new = 255 - old`,
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
            description: "Inverts the colors of an image.".to_string(),
        }
    }

    /// Creates the input port: a single image to invert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the output port: the color-inverted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the invert operation. Attempts to unwrap the Arc to avoid cloning when possible.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node
        // Try to take ownership of the image data to avoid cloning; fall back to clone if shared
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        data_inner.invert();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(data_inner), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "invert_tests.rs"]
mod tests;
