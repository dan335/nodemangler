//! Levels adjustment operation for images.
//!
//! Remaps pixel values using input levels (low, mid, high) and output levels
//! (low, high). The input range is contracted or expanded, the midtone control
//! reshapes the gamma curve, and the output range scales the final result.
//! Matches the Substance Designer levels node behavior.

use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Levels adjustment operation with input low/mid/high and output low/high controls.
///
/// The midtone parameter uses a 0–1 scale where 0.5 is neutral, matching
/// Substance Designer's convention. Internally this is converted to a gamma
/// exponent via `gamma = log(0.5) / log(midtone)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentLevels{}

impl OpImageAdjustmentLevels {
    /// Returns the node metadata (name and description) for the levels operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "levels".to_string(),
            description: "Adjusts input levels (low/mid/high) and output range.".to_string(),
        }
    }

    /// Creates the input ports: image, input low/mid/high, and output low/high.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("in low".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("in mid".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 0.99), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("in high".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("out low".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("out high".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the levels-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Converts a midtone value (0–1, 0.5 = neutral) to a gamma exponent.
    ///
    /// Uses the standard formula: `gamma = log(0.5) / log(midtone)`.
    /// At midtone = 0.5, gamma = 1.0 (identity).
    /// Below 0.5, gamma < 1 (darkens midtones).
    /// Above 0.5, gamma > 1 (brightens midtones).
    fn midtone_to_gamma(midtone: f32) -> f32 {
        let midtone = midtone.clamp(0.01, 0.99);
        (0.5_f32).ln() / midtone.ln()
    }

    /// Executes the levels adjustment. Operates in 32-bit float space for precision.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let in_low_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let in_mid_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let in_high_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let out_low_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let out_high_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(in_low) = in_low_converted.unwrap() else { unreachable!() };
        let Value::Decimal(in_mid) = in_mid_converted.unwrap() else { unreachable!() };
        let Value::Decimal(in_high) = in_high_converted.unwrap() else { unreachable!() };
        let Value::Decimal(out_low) = out_low_converted.unwrap() else { unreachable!() };
        let Value::Decimal(out_high) = out_high_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        // Prevent division by zero when input low and high are equal
        let in_range = (in_high - in_low).max(0.001);
        let gamma = Self::midtone_to_gamma(in_mid);
        let inv_gamma = 1.0 / gamma;

        for pixel in buffer.pixels_mut() {
            for c in 0..3 {
                let val = pixel[c];
                // Remap from [in_low, in_high] to [0, 1]
                let remapped = ((val - in_low) / in_range).clamp(0.0, 1.0);
                // Apply gamma correction
                let corrected = remapped.powf(inv_gamma);
                // Remap to [out_low, out_high]
                pixel[c] = out_low + corrected * (out_high - out_low);
            }
            // alpha unchanged
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "levels_tests.rs"]
mod tests;
