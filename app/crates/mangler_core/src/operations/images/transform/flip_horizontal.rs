//! Horizontal flip (mirror left-to-right) operation.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Flips an image horizontally (mirrors left-to-right).
///
/// The operation is performed in-place when possible (single `Arc` reference),
/// otherwise the image data is cloned first. Applying this operation twice
/// restores the original image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformFlipHorizontal {}

impl OpImageTransformFlipHorizontal {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flip horizontal".to_string(),
            description: "Flips an image horizontally.".to_string(),
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

    /// Executes the horizontal flip operation in-place.
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
        image::imageops::flip_horizontal_in_place(&mut data_inner);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(data_inner), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "flip_horizontal_tests.rs"]
mod tests;
