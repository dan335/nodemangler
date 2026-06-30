//! White balance: temperature and tint correction.
//!
//! Applies multiplicative per-channel gains. Temperature trades red against
//! blue (warm ↔ cool); tint trades green against magenta. Grayscale inputs
//! have no chroma to balance and pass through unchanged.

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

/// How strongly the temperature / tint controls scale their channels at ±1.
const STRENGTH: f32 = 0.3;

/// White balance adjustment via temperature and tint channel gains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentWhiteBalance {}

impl OpImageAdjustmentWhiteBalance {
    /// Returns the node metadata (name and description) for white balance.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "white balance".to_string(),
            description: "Corrects colour temperature (warm/cool) and tint (green/magenta).".to_string(),
            help: "Applies multiplicative per-channel gains. Temperature scales red by 1 + temp*0.3 and blue by 1 - temp*0.3, so positive values warm the image (more red, less blue) and negative values cool it. Tint scales green by 1 - tint*0.3, so positive tint pushes toward magenta and negative toward green.\n\nBoth controls are normalized to [-1, 1]. Results are not clamped, so strong corrections can push channels outside 0-1 for downstream nodes to handle. Grayscale inputs (1 or 2 channels) carry no chroma and pass through unchanged; alpha is always preserved.".to_string(),
        }
    }

    /// Creates input ports: image, temperature, and tint.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to white-balance."),
            Input::new("temperature".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Warm/cool shift; positive warms (red up, blue down)."),
            Input::new("tint".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Green/magenta shift; positive pushes toward magenta."),
        ]
    }

    /// Creates the output port: the white-balanced image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with temperature and tint gains applied."),
        ]
    }

    /// Executes the white balance by scaling the R/G/B channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let temp_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let tint_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(temperature) = temp_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tint) = tint_converted.unwrap() else { unreachable!() };

        let ch = data.channels() as usize;
        if ch < 3 {
            // Grayscale: nothing to balance.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let r_gain = 1.0 + temperature * STRENGTH;
        let g_gain = 1.0 - tint * STRENGTH;
        let b_gain = 1.0 - temperature * STRENGTH;

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            pixel[0] *= r_gain;
            pixel[1] *= g_gain;
            pixel[2] *= b_gain;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "white_balance_tests.rs"]
mod tests;
