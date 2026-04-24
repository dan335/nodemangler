//! Levels adjustment operation for images.
//!
//! Remaps pixel values using input levels (low, mid, high) and output levels
//! (low, high). The input range is contracted or expanded, the midtone control
//! reshapes the gamma curve, and the output range scales the final result.
//! Matches the Substance Designer levels node behavior.

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

/// Levels adjustment operation with input low/mid/high and output low/high controls.
///
/// The midtone parameter uses a 0-1 scale where 0.5 is neutral, matching
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
            help: "Matches the Substance Designer levels node: each colour channel is first clamped and normalised from [in low, in high] to 0-1, then passed through a gamma curve driven by the midtone (0.5 = neutral, using gamma = log(0.5)/log(midtone)), and finally scaled into [out low, out high].\n\nIn mid values below 0.5 brighten midtones, values above darken them. When in low and in high are equal the input range is clamped to a minimum of 0.001 to avoid division by zero. Alpha is preserved. Use this as your main tonal workhorse: black/white point correction, gamma, and output range in one pass.".to_string(),
        }
    }

    /// Creates the input ports: image, input low/mid/high, and output low/high.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to remap through the levels curve."),
            Input::new("in low".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Input black point; values at or below this become 0."),
            Input::new("in mid".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 0.99), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Midtone pivot; values below 0.5 brighten midtones, above darken them."),
            Input::new("in high".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Input white point; values at or above this become 1."),
            Input::new("out low".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Output black point; the darkest values are mapped to this."),
            Input::new("out high".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Output white point; the brightest values are mapped to this."),
        ]
    }

    /// Creates the output port: the levels-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image remapped through the input/output levels and midtone gamma."),
        ]
    }

    /// Converts a midtone value (0-1, 0.5 = neutral) to a gamma exponent.
    ///
    /// Uses the standard formula: `gamma = log(0.5) / log(midtone)`.
    fn midtone_to_gamma(midtone: f32) -> f32 {
        let midtone = midtone.clamp(0.01, 0.99);
        (0.5_f32).ln() / midtone.ln()
    }

    /// Executes the levels adjustment directly on FloatImage data.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let in_low_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let in_mid_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let in_high_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let out_low_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let out_high_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(in_low) = in_low_converted.unwrap() else { unreachable!() };
        let Value::Decimal(in_mid) = in_mid_converted.unwrap() else { unreachable!() };
        let Value::Decimal(in_high) = in_high_converted.unwrap() else { unreachable!() };
        let Value::Decimal(out_low) = out_low_converted.unwrap() else { unreachable!() };
        let Value::Decimal(out_high) = out_high_converted.unwrap() else { unreachable!() };

        // run node — data is already f32, clone and work directly
        let mut result = (*data).clone();
        // Prevent division by zero when input low and high are equal
        let in_range = (in_high - in_low).max(0.001);
        let gamma = Self::midtone_to_gamma(in_mid);
        let inv_gamma = 1.0 / gamma;
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for c in 0..color_ch {
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

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "levels_tests.rs"]
mod tests;
