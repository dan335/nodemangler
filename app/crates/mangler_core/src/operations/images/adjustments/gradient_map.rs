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
use rayon::prelude::*;
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
            help: "Computes each pixel's Rec. 709 luminance, then uses that value as a 0-1 parameter along a colour ramp. In two-colour mode, output is a straight lerp from A at 0 to B at 1; in three-colour mode, a middle colour C is inserted at the configurable mid position, creating an A-to-C lerp below and a C-to-B lerp above.\n\nColours are pulled from sRGB floats including alpha on the gradient, but the source image's own alpha is preserved. Output is always a 4-channel RGBA image. Useful for recolouring height or mask fields and for mood-shifting photos.".to_string(),
        }
    }

    /// Creates the input ports: image, two endpoint colors (a, b), an optional mid color (c),
    /// a toggle for using the mid color, and a mid position slider.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose luminance picks a position along the gradient."),
            Input::new("color a".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Colour at the dark end of the gradient (luminance 0)."),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None)
                .with_description("Colour at the bright end of the gradient (luminance 1)."),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None)
                .with_description("Optional middle colour inserted at the mid position when enabled."),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None)
                .with_description("When on, interpolates A→C→B instead of a plain two-colour ramp."),
            Input::new("mid position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance position where colour C sits along the gradient."),
        ]
    }

    /// Creates the output port: the gradient-mapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA image recoloured by mapping luminance through the gradient."),
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

        let img = &*data;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            (0..width).flat_map(move |x| {
                let px = img.get_pixel(x, y);
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

                [out_r, out_g, out_b, original_a]
            })
        }).collect();

        let output = FloatImage::from_raw(width, height, 4, pixels).unwrap();

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
