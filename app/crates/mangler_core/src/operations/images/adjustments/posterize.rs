//! Posterize (color quantization) operation for images.
//!
//! Reduces the number of discrete color levels per channel, creating a
//! banded or poster-like appearance. With 2 levels, the output is pure
//! black and white.

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

/// Posterize operation that quantizes pixel values to a limited number of discrete levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentPosterize {}

impl OpImageAdjustmentPosterize {
    /// Returns the node metadata (name and description) for the posterize operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "posterize".to_string(),
            description: "Reduces color depth to a specified number of levels per channel.".to_string(),
        }
    }

    /// Creates the input ports: image and number of quantization levels (2-256).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("levels".to_string(), Value::Integer(4), Some(InputSettings::DragValue { speed: None, clamp: Some((2.0, 256.0)) }), None),
        ]
    }

    /// Creates the output port: the posterized image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the posterize operation. Quantizes each non-alpha channel to the specified number of levels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        // run node — quantize each non-alpha channel
        let mut result = (*data).clone();
        let levels = (levels as f32).max(2.0);
        let steps = levels - 1.0;
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for c in 0..color_ch {
                let val = pixel[c];
                // Round to nearest quantization step
                let quantized = (val * steps + 0.5).floor() / steps;
                pixel[c] = quantized.clamp(0.0, 1.0);
            }
            // alpha unchanged
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "posterize_tests.rs"]
mod tests;
