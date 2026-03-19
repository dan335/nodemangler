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
use image::DynamicImage;

/// Posterize operation that quantizes pixel values to a limited number of discrete levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentPosterize {}

impl OpImageAdjustmentPosterize {
    /// Returns the node metadata (name and description) for the posterize operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "posterize".to_string(),
            description: "Reduces the number of color levels.".to_string(),
        }
    }

    /// Creates the input ports: image and number of quantization levels (2-256).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("levels".to_string(), Value::Integer(4), Some(InputSettings::DragValue { speed: None, clamp: Some((2.0, 256.0)) }), None),
        ]
    }

    /// Creates the output port: the posterized image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the posterize operation. Quantizes each channel to the specified number of levels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let levels = (levels as f32).max(2.0);
        let steps = levels - 1.0;

        for pixel in buffer.pixels_mut() {
            for c in 0..3 {
                let val = pixel[c];
                // Round to nearest quantization step
                let quantized = (val * steps + 0.5).floor() / steps;
                pixel[c] = quantized.clamp(0.0, 1.0);
            }
            // alpha unchanged
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(adjusted), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "posterize_tests.rs"]
mod tests;
