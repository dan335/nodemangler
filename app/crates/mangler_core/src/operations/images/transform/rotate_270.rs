//! 270-degree clockwise (90-degree counter-clockwise) rotation operation.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Rotates an image 270 degrees clockwise (equivalent to 90 degrees counter-clockwise).
///
/// The output dimensions are swapped: width becomes height and vice versa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformRotate270 {}

impl OpImageTransformRotate270 {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate 270".to_string(),
            description: "Rotates an image 270 degrees.".to_string(),
        }
    }

    /// Creates the default inputs: a single source image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the default outputs: the rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the 270-degree clockwise rotation.
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
        let im = data.rotate270();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(im), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "rotate_270_tests.rs"]
mod tests;
