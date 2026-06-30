//! Selective color: adjust a targeted hue band's hue, saturation, and lightness.
//!
//! Pixels are converted to HSL; their angular distance from a target hue drives
//! a smooth weight over a configurable band. Within the band the hue, saturation,
//! and lightness deltas are applied proportionally to the weight. Pixels outside
//! the band (and grayscale inputs) are untouched.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use super::common::{hsl_to_rgb, rgb_to_hsl, smoothstep};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Targeted hue-band HSL adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSelectiveColor {}

impl OpImageAdjustmentSelectiveColor {
    /// Returns the node metadata (name and description) for selective color.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "selective color".to_string(),
            description: "Adjusts the hue, saturation, and lightness of one targeted colour range.".to_string(),
            help: "Converts each pixel to HSL and measures the shortest angular distance from `target hue`. A smoothstep over `range` degrees produces a per-pixel weight (1 at the target hue, fading to 0 at the band edge). Inside the band the deltas are applied proportionally: hue is shifted by weight * shift * 180 degrees, while saturation and lightness add weight * their slider and clamp to 0-1.\n\nThis isolates, say, the reds to recolour or desaturate them while leaving other hues alone. Target hue is in degrees (0 = red, 120 = green, 240 = blue) and range is the half-width of the selection. Grayscale inputs (1 or 2 channels) have no hue and pass through unchanged; alpha is preserved.".to_string(),
        }
    }

    /// Creates input ports: image, target hue, band width, and H/S/L deltas.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to selectively adjust."),
            Input::new("target hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Centre hue of the selection in degrees (0 red, 120 green, 240 blue)."),
            Input::new("range".to_string(), Value::Decimal(30.0), Some(InputSettings::Slider { range: (0.0, 180.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-width of the hue band in degrees over which the effect fades out."),
            Input::new("hue shift".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Hue rotation within the band, normalized to ±180 degrees."),
            Input::new("saturation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Saturation delta added within the band (clamped 0-1)."),
            Input::new("lightness".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Lightness delta added within the band (clamped 0-1)."),
        ]
    }

    /// Creates the output port: the selectively adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with the targeted hue band adjusted."),
        ]
    }

    /// Executes the selective colour adjustment over the targeted hue band.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let target_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let range_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let hue_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let sat_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let light_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(target_hue) = target_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range) = range_converted.unwrap() else { unreachable!() };
        let Value::Decimal(hue_shift) = hue_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation) = sat_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lightness) = light_converted.unwrap() else { unreachable!() };

        let ch = data.channels() as usize;
        if ch < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let range = range.max(1e-3);
        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let (h, s, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            // Shortest angular distance on the 0-360 hue wheel.
            let mut diff = (h - target_hue).abs() % 360.0;
            if diff > 180.0 { diff = 360.0 - diff; }
            let weight = 1.0 - smoothstep(0.0, range, diff);
            if weight <= 0.0 {
                continue;
            }
            let nh = (h + weight * hue_shift * 180.0).rem_euclid(360.0);
            let ns = (s + weight * saturation).clamp(0.0, 1.0);
            let nl = (l + weight * lightness).clamp(0.0, 1.0);
            let (r, g, b) = hsl_to_rgb(nh, ns, nl);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "selective_color_tests.rs"]
mod tests;
