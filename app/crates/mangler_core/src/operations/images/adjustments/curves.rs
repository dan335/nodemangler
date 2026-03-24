//! Tone curve adjustment operation for images.
//!
//! Applies a contrast-like curve centered on a configurable midpoint.
//! Positive strength increases contrast (S-curve), negative strength
//! reduces contrast around the midpoint.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Tone curve adjustment that applies contrast scaling around a configurable midpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentCurves{}

impl OpImageAdjustmentCurves {
    /// Returns the node metadata (name and description) for the curves operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curves".to_string(),
            description: "Applies a tone curve adjustment.".to_string(),
        }
    }

    /// Creates the input ports: image, strength (-1..1), and midpoint (0..1).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("strength".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the curve-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the curves adjustment. Applies a linear contrast curve centered on the midpoint.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let strength_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let midpoint_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(strength) = strength_converted.unwrap() else { unreachable!() };
        let Value::Decimal(midpoint) = midpoint_converted.unwrap() else { unreachable!() };

        // run node — clone the FloatImage and apply contrast curve
        let mut result = (*data).clone();
        // Double the strength to get a more perceptually useful contrast range
        let contrast = strength * 2.0;
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for c in 0..color_ch {
                let val = pixel[c];
                // Scale deviation from midpoint by the contrast factor
                let adjusted = midpoint + (val - midpoint) * (1.0 + contrast);
                pixel[c] = adjusted.clamp(0.0, 1.0);
            }
            // alpha unchanged
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "curves_tests.rs"]
mod tests;
