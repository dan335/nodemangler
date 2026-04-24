//! Brightness adjustment operation for images.
//!
//! Adjusts image brightness by adding a fixed offset to every non-alpha channel.
//! The amount is in the normalized range (-1..1) and is scaled to (-1..1) f32 offset.

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

/// Brightness adjustment operation that adds a constant offset to pixel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentBrighten{}

impl OpImageAdjustmentBrighten {
    /// Returns the node metadata (name and description) for the brighten operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "brighten".to_string(),
            description: "Adjusts image brightness. Positive brightens, negative darkens.".to_string(),
            help: "Adds a constant offset to every colour channel of each pixel. The amount is a signed decimal in the -1 to 1 range; 0 leaves the image untouched, +1 pushes all channels up by one unit, -1 pulls them all down.\n\nAlpha is skipped so transparency is preserved. Values are not clamped, so extreme amounts can push channels outside the 0-1 interval and clip on later stages that expect normalised data. For a perceptual brightness change use a levels or curves node instead; this one is purely additive.".to_string(),
        }
    }

    /// Creates the input ports: an image and an amount (-1.0 to 1.0) controlling brightness offset.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to brighten or darken."),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Offset added to every colour channel; positive brightens, negative darkens.")
        ]
    }

    /// Creates the output port: the brightness-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with the brightness offset applied, alpha preserved."),
        ]
    }

    /// Executes the brighten operation. Adds the amount directly to each non-alpha channel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };

        // run node — clone the FloatImage and add brightness offset to each non-alpha channel
        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        // Determine how many color channels to adjust (skip alpha if present)
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for c in 0..color_ch {
                pixel[c] += amount;
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
#[path = "brighten_tests.rs"]
mod tests;
