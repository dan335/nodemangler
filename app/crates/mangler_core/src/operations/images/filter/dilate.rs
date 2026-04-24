//! Morphological dilation.
//!
//! For each pixel, replaces it with the per-channel maximum over a square
//! neighborhood. Dilation grows bright regions and shrinks dark ones; it is
//! the dual of erosion and pairs with it to form open/close operators.
//!
//! The separable min/max implementation lives in `erode.rs` and is reused
//! here with `f32::max` as the reducer.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::filter::erode::separable_morphology;
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Morphological dilation (per-channel max in a square window).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDilate {}

impl OpImageAdjustmentDilate {
    /// Returns the node metadata (name and description) for dilate.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "dilate".to_string(),
            description: "Morphological dilation — per-channel max in a square neighborhood. Grows bright regions.".to_string(),
            help: "For each pixel takes the per-channel maximum over a (2r+1) square window. Bright regions grow by `radius` pixels, dark regions shrink, small dark pits are filled, and isolated bright pixels spread.\n\nDual of erosion; combining the two yields open/close. Implemented as two 1D sweeps (horizontal then vertical) via the separable morphology primitive in `erode.rs`, so cost is O(r) per pixel.".to_string(),
        }
    }

    /// Creates input ports: image and radius (square window half-size).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask to dilate."),
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window in pixels; larger values grow bright regions more."),
        ]
    }

    /// Creates the output port: the dilated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Dilated image where bright regions have expanded by the chosen radius."),
        ]
    }

    /// Runs the dilation operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        let radius = radius.max(1) as i32;

        // Reuse the separable pass from erode.rs with max as the reducer
        let out = separable_morphology(&data, radius, |a, b| a.max(b));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "dilate_tests.rs"]
mod tests;
