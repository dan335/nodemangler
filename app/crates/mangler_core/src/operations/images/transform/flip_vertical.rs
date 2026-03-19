//! Vertical flip (mirror top-to-bottom) operation.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Flips an image vertically (mirrors top-to-bottom).
///
/// The operation is performed in-place when possible. Applying this operation
/// twice restores the original image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformFlipVertical {}

impl OpImageTransformFlipVertical {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flip vertical".to_string(),
            description: "Flips an image vertically.".to_string(),
        }
    }

    /// Creates the default inputs: a single source image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the default outputs: the flipped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the vertical flip operation in-place.
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
        // Try to take ownership; clone if other references exist
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        image::imageops::flip_vertical_in_place(&mut data_inner);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(data_inner), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "flip_vertical_tests.rs"]
mod tests;
