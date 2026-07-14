//! Photoshop-style Exposure adjustment operation for images.
//!
//! Mirrors Adobe Photoshop's Exposure adjustment. Each non-alpha channel value `v`
//! is transformed in three stages:
//!   1. Exposure — a multiplicative gain of `2^stops` (measured in photographic stops).
//!   2. Offset   — an additive shift that lifts or lowers the darker tones.
//!   3. Gamma    — a power correction `pow(v, 1/gamma)` that reshapes the midtones.
//! The result is left unclamped f32 (the pipeline works in unbounded float).

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

/// Photoshop-style Exposure adjustment: multiplicative exposure (in stops), additive
/// offset, then a gamma power correction, applied to each non-alpha channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentExposure{}

impl OpImageAdjustmentExposure {
    /// Returns the node metadata (name, description, and help) for the exposure operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "exposure".to_string(),
            description: "Photoshop-style exposure: exposure (stops), offset, then gamma.".to_string(),
            help: "Reproduces Adobe Photoshop's Exposure adjustment. Each colour channel value v \
                   is processed in three stages, in this order:\n\n\
                   1. Exposure — v is multiplied by 2^exposure. The exposure control is measured \
                   in photographic stops, so +1 doubles the linear value and -1 halves it. This \
                   is a purely multiplicative gain that mostly affects the brighter tones.\n\n\
                   2. Offset — the offset value is added to v. This additive shift lifts or lowers \
                   the shadows/blacks with little effect on the highlights.\n\n\
                   3. Gamma — v is raised to the power 1/gamma (after guarding against negatives, \
                   so the base is never below 0). Gamma greater than 1 lightens the midtones; less \
                   than 1 darkens them.\n\n\
                   Alpha is never touched, so transparency is preserved. Grayscale and single/\
                   double-channel images are processed on their luma/colour channels too. The final \
                   result is intentionally left UNCLAMPED — values may fall outside the 0-1 range \
                   and will clip only on later stages that expect normalised data. This matches how \
                   an exposure control behaves on high-dynamic-range data.".to_string(),
        }
    }

    /// Creates the input ports: the source image, exposure (stops), offset, and gamma.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to expose."),
            // Exposure in photographic stops; applied multiplicatively as 2^exposure.
            Input::new("exposure".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-5.0, 5.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Exposure in stops. Applied as a 2^exposure multiplier; +1 doubles the value, -1 halves it."),
            // Additive offset that lifts/lowers the darker tones.
            Input::new("offset".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.001), clamp_to_range: false }), None)
                .with_description("Additive offset applied after exposure; lifts or lowers the shadows."),
            // Gamma power correction; final value is pow(v, 1/gamma).
            Input::new("gamma".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 5.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Gamma correction; final value is pow(v, 1/gamma). Above 1 lightens midtones, below 1 darkens."),
        ]
    }

    /// Creates the output port: the exposure-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with exposure, offset, and gamma applied; alpha preserved, values unclamped."),
        ]
    }

    /// Executes the exposure operation: multiplicative exposure, additive offset, then gamma.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted    = convert_input(inputs, 0, ValueType::Image,   &mut input_errors);
        let exposure_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let offset_converted   = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let gamma_converted    = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exposure) = exposure_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset)   = offset_converted.unwrap() else { unreachable!() };
        let Value::Decimal(gamma)    = gamma_converted.unwrap() else { unreachable!() };

        // Pre-compute the constant factors used per pixel.
        let exposure_gain = 2f32.powf(exposure as f32); // 2^stops multiplicative gain
        let offset = offset as f32;                      // additive shadow lift
        // Guard gamma so we never divide by zero; the slider clamps to >= 0.1 anyway.
        let inv_gamma = 1.0 / (gamma as f32).max(1e-6);   // exponent for the gamma stage

        // run node — clone the FloatImage and apply the exposure transform per non-alpha channel
        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        // Determine how many color channels to adjust (skip alpha if present)
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for val in pixel.iter_mut().take(color_ch) {
                // 1. exposure (multiplicative, in stops)
                let mut v = *val * exposure_gain;
                // 2. offset (additive)
                v += offset;
                // 3. gamma correction — guard the base against negatives before powf
                *val = v.max(0.0).powf(inv_gamma);
            }
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
#[path = "exposure_tests.rs"]
mod tests;
