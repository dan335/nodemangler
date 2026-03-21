//! Gradient map operation for images.
//!
//! Maps each pixel's luminance to a position on a color gradient, replacing
//! the original color. Supports two-color or three-color gradients with a
//! configurable midpoint position.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Gradient map operation that recolors an image by mapping luminance to a color gradient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGradientMap {}

impl OpImageAdjustmentGradientMap {
    /// Returns the node metadata (name and description) for the gradient map operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gradient map".to_string(),
            description: "Maps image luminance to a color gradient.".to_string(),
        }
    }

    /// Creates the input ports: image, two endpoint colors (a, b), an optional mid color (c),
    /// a toggle for using the mid color, and a mid position slider.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("color a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the gradient-mapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the gradient map. Computes Rec. 709 luminance per pixel and interpolates
    /// between gradient colors based on luminance position. Output is always 4-channel RGBA.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let color_a_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let color_b_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);
        let color_c_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);
        let use_mid_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);
        let mid_pos_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Color(color_a) = color_a_converted.unwrap() else { unreachable!() };
        let Value::Color(color_b) = color_b_converted.unwrap() else { unreachable!() };
        let Value::Color(color_c) = color_c_converted.unwrap() else { unreachable!() };
        let Value::Bool(use_mid) = use_mid_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mid_pos) = mid_pos_converted.unwrap() else { unreachable!() };

        // run node — compute luminance and map to gradient colors
        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        let (ar, ag, ab, aa) = color_a.to_srgb_float();
        let (br, bg, bb, ba) = color_b.to_srgb_float();
        let (cr, cg, cb, ca) = color_c.to_srgb_float();

        let mut output = FloatImage::new(width, height, 4);

        for y in 0..height {
            for x in 0..width {
                let px = data.get_pixel(x, y);
                // Compute luminance from available channels
                let (r, g, b) = if ch >= 3 {
                    (px[0], px[1], px[2])
                } else {
                    (px[0], px[0], px[0])
                };
                let original_a = if ch == 2 || ch == 4 { px[ch - 1] } else { 1.0 };

                // Rec. 709 luminance
                let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0);

                let (out_r, out_g, out_b, _out_a) = if use_mid {
                    // Three-color gradient: lerp A->C below midpoint, C->B above midpoint
                    if lum < mid_pos {
                        let t = if mid_pos > 0.0 { lum / mid_pos } else { 0.0 };
                        (ar + (cr - ar) * t, ag + (cg - ag) * t, ab + (cb - ab) * t, aa + (ca - aa) * t)
                    } else {
                        let t = if mid_pos < 1.0 { (lum - mid_pos) / (1.0 - mid_pos) } else { 1.0 };
                        (cr + (br - cr) * t, cg + (bg - cg) * t, cb + (bb - cb) * t, ca + (ba - ca) * t)
                    }
                } else {
                    // Two-color gradient: simple linear interpolation A->B
                    (ar + (br - ar) * lum, ag + (bg - ag) * lum, ab + (bb - ab) * lum, aa + (ba - aa) * lum)
                };

                output.put_pixel(x, y, &[out_r, out_g, out_b, original_a]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data: Arc::new(output), change_id: get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "gradient_map_tests.rs"]
mod tests;
