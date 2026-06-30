//! Hue rotation operation for images.
//!
//! Rotates the hue of all pixels by a specified amount. The input amount is
//! normalized (-1..1) and mapped to degrees (-360..360). Converts each pixel
//! to HSL, adds the rotation, and converts back. For 1-channel images, returns as-is.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use super::common::{hsl_to_rgb, rgb_to_hsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Hue rotation operation that shifts pixel hue angles by a specified amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHueRotate{}

impl OpImageAdjustmentHueRotate {
    /// Returns the node metadata (name and description) for the hue rotate operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hue shift".to_string(),
            description: "Rotates the hue of an image.".to_string(),
            help: "For every RGB pixel, converts to HSL, adds amount * 360 degrees to the hue, then wraps it back into 0-360 before converting back to RGB. Saturation and lightness are preserved exactly, so the image's tonality is untouched while colours slide around the wheel.\n\nImages with fewer than three colour channels (1 or 2 channel grayscale) have no hue to rotate and are passed through unchanged. Alpha, when present, is left intact. The amount input is normalised to -1 to 1 so a full spin equals 1 and sign picks rotation direction.".to_string(),
        }
    }

    /// Creates the input ports: an image and a normalized rotation amount (-1.0 to 1.0, mapped to -360..360 degrees).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Colour image whose hues will be rotated."),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Hue shift normalised to [-1, 1], corresponding to -360 to 360 degrees.")
        ]
    }

    /// Creates the output port: the hue-rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with hues rotated around the colour wheel; saturation and lightness are preserved."),
        ]
    }

    /// Executes the hue rotation. Converts each pixel RGB->HSL, adds degrees, converts back.
    /// For 1-channel images (grayscale), returns as-is since there is no hue to rotate.
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

        // run node
        let ch = data.channels() as usize;
        if ch < 3 {
            // 1 or 2 channel image (grayscale), no hue to rotate
            return Ok(OperationResponse { 
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse {value: Value::Image { data, change_id:get_id() }},
                ],
            });
        }

        let degrees = amount * 360.0;
        let mut result = (*data).clone();

        for pixel in result.pixels_mut() {
            // Convert RGB to HSL
            let (h, s, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            // Rotate hue, wrapping around 0..360
            let new_h = (h + degrees).rem_euclid(360.0);
            // Convert back to RGB
            let (r, g, b) = hsl_to_rgb(new_h, s, l);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            // Alpha (if present) is unchanged
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
#[path = "hue_rotate_tests.rs"]
mod tests;
