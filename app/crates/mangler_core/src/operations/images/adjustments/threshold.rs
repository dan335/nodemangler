//! Luminance threshold (binarize) operation.
//!
//! Converts an image to two tones by comparing each pixel's luminance against
//! a threshold. A `smoothness` parameter softens the cut into a smoothstep
//! ramp instead of a hard edge. Alpha is preserved; colour channels are all
//! set to the same binary/ramp value, producing a grayscale mask.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use super::common::smoothstep;
use rayon::prelude::*;
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
            image_input("image")
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
            image_output("output")
                .with_description("Two-tone (or soft-ramped) grayscale mask; alpha preserved."),
        ]
    }

    /// Executes the threshold, writing the binary/ramp value to all colour channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(threshold) = 1,
            Decimal(smoothness) = 2,
        }

        let ch = data.channels();
        let color_ch = (if ch == 2 || ch == 4 { ch - 1 } else { ch }) as usize;

        let mut result = (*data).clone();
        result.par_pixels_mut().for_each(|pixel| {
            let luma = if color_ch >= 3 {
                crate::luma::rec709(pixel[0], pixel[1], pixel[2])
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
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "threshold_tests.rs"]
mod tests;
