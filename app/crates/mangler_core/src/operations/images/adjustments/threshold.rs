//! Luminance threshold (binarize) operation.
//!
//! Converts an image to two tones by comparing each pixel's luminance against
//! a threshold. A `smoothness` parameter softens the cut into a smoothstep
//! ramp instead of a hard edge. Alpha is preserved; colour channels are all
//! set to the same binary/ramp value, producing a grayscale mask.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use super::common::smoothstep;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Binarize an image by luminance with an optional soft transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentThreshold {}

impl OpImageAdjustmentThreshold {
    /// Returns the node metadata (name and description) for threshold.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "threshold".to_string(),
            description: "Binarizes by luminance: pixels above the threshold become white, below become black.".to_string(),
            help: "Computes each pixel's Rec. 709 luminance (or the single channel value for grayscale inputs) and compares it to `threshold`. With smoothness at 0 the result is a hard two-tone mask: 1.0 at or above the threshold, 0.0 below. Raising smoothness replaces the hard cut with a smoothstep ramp spanning threshold ± smoothness, giving anti-aliased edges.\n\nAll colour channels receive the same value, so the output is an achromatic mask regardless of input channel count; alpha is passed through untouched. Useful for extracting masks from gradients, noise, and height fields before morphology or compositing.".to_string(),
        }
    }

    /// Creates input ports: image, threshold level, and edge smoothness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to binarize by luminance."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff; pixels at or above this become white."),
            Input::new("smoothness".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Half-width of the soft transition; 0 is a hard edge."),
        ]
    }

    /// Creates the output port: the thresholded mask.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Two-tone (or soft-ramped) grayscale mask; alpha preserved."),
        ]
    }

    /// Executes the threshold, writing the binary/ramp value to all colour channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let smoothness_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(smoothness) = smoothness_converted.unwrap() else { unreachable!() };

        let ch = data.channels();
        let color_ch = (if ch == 2 || ch == 4 { ch - 1 } else { ch }) as usize;

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let luma = if color_ch >= 3 {
                0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
            } else {
                pixel[0]
            };
            let v = if smoothness <= 0.0 {
                if luma >= threshold { 1.0 } else { 0.0 }
            } else {
                smoothstep(threshold - smoothness, threshold + smoothness, luma)
            };
            for val in pixel.iter_mut().take(color_ch) {
                *val = v;
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "threshold_tests.rs"]
mod tests;
