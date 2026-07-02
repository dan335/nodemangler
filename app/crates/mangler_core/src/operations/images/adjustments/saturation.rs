//! Saturation adjustment for images.
//!
//! Scales each pixel's chroma by interpolating between its luminance (a fully
//! desaturated gray) and the original colour. `amount = 0` yields grayscale,
//! `1` is identity, and values above `1` boost saturation. Uses Rec. 709
//! luminance weights and leaves alpha untouched.

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

/// Saturation adjustment that scales chroma around per-pixel luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSaturation {}

impl OpImageAdjustmentSaturation {
    /// Returns the node metadata (name and description) for saturation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "saturation".to_string(),
            description: "Adjusts colour saturation. 0 is grayscale, 1 is identity, >1 boosts.".to_string(),
            help: "Computes each pixel's Rec. 709 luminance and linearly interpolates between that gray and the original colour: output = luma + (colour - luma) * amount. An amount of 0 collapses every channel to luminance (full grayscale), 1 is the identity, and values above 1 push colours away from gray to boost vividness.\n\nLuminance is preserved at every setting because the interpolation pivots around it, so brightness does not drift as saturation changes. Alpha is left untouched and results are not clamped, so large amounts can push channels outside 0-1. Grayscale inputs (1 or 2 channels) have no chroma and pass through unchanged.".to_string(),
        }
    }

    /// Creates input ports: source image and a saturation multiplier.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to saturate or desaturate."),
            Input::new("amount".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Saturation multiplier; 0 grayscale, 1 identity, >1 more vivid."),
        ]
    }

    /// Creates the output port: the saturation-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with chroma scaled around its luminance."),
        ]
    }

    /// Executes the saturation adjustment by lerping each colour channel toward luminance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };

        let ch = data.channels() as usize;
        if ch < 3 {
            // Grayscale: no chroma to scale, pass through unchanged.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let luma = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            for val in pixel.iter_mut().take(3) {
                *val = luma + (*val - luma) * amount;
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "saturation_tests.rs"]
mod tests;
