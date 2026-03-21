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
        }
    }

    /// Creates the input ports: an image and a normalized rotation amount (-1.0 to 1.0, mapped to -360..360 degrees).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
        ]
    }

    /// Creates the output port: the hue-rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
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

/// Converts an RGB color (each in 0..1) to HSL (hue in 0..360, s/l in 0..1).
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-7 {
        // Achromatic
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

    let h = if (max - r).abs() < 1e-7 {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-7 {
        ((b - r) / d) + 2.0
    } else {
        ((r - g) / d) + 4.0
    };

    (h * 60.0, s, l)
}

/// Converts an HSL color (hue in 0..360, s/l in 0..1) back to RGB (each in 0..1).
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < 1e-7 {
        return (l, l, l);
    }

    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;

    let r = hue_to_rgb(p, q, h_norm + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h_norm);
    let b = hue_to_rgb(p, q, h_norm - 1.0 / 3.0);

    (r, g, b)
}

/// Helper for HSL->RGB conversion: maps a hue sector to an RGB component.
fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

#[cfg(test)]
#[path = "hue_rotate_tests.rs"]
mod tests;
