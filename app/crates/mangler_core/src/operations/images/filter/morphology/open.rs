//! Morphological opening: erosion followed by dilation.
//!
//! Removes small bright specks and thin bright details while preserving the
//! overall size of larger bright regions. Pairs with `close` for noise cleanup
//! on masks.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::filter::morphology::erode::separable_morphology;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Morphological opening (erode then dilate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentOpen {}

impl OpImageAdjustmentOpen {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "open".to_string(),
            description: "Morphological opening — erode then dilate. Removes small bright specks.".to_string(),
            help: "Runs an erosion (per-channel min in a square window) immediately followed by a dilation (per-channel max) using the same radius. Bright specks, thin bright filaments, and isolated highlights smaller than the structuring element disappear while larger bright regions recover their original footprint.\n\nComplementary to `close` for mask cleanup. Uses the separable morphology primitive from `erode.rs` so cost is O(r) per pixel.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask to open."),
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Structuring element radius in pixels; larger values remove bigger bright specks."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image after morphological opening that removes small bright specks."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        let radius = radius.max(1);
        let eroded = separable_morphology(&data, radius, |a, b| a.min(b));
        let opened = separable_morphology(&eroded, radius, |a, b| a.max(b));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(opened), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
