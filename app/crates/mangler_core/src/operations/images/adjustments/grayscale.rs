//! Grayscale conversion operation for images.
//!
//! Converts an image to grayscale using the `image` crate's built-in
//! luminance-weighted conversion, producing equal R, G, B channels.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Grayscale conversion operation that removes color information from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGrayscale {}

impl OpImageAdjustmentGrayscale {
    /// Returns the node metadata (name and description) for the grayscale operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grayscale".to_string(),
            description: "Converts an image to grayscale.".to_string(),
        }
    }

    /// Creates the input port: a single image to convert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the output port: the grayscale-converted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id:get_id() }, None),
        ]
    }

    /// Executes the grayscale conversion on the input image.
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
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(data.grayscale()), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "grayscale_tests.rs"]
mod tests;
