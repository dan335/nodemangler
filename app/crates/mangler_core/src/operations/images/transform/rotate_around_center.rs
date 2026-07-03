//! Arbitrary-angle rotation around the image center.
//!
//! Inverse-transforms each output pixel and bilinear-samples the source
//! [`FloatImage`] directly in f32, so no 8-bit quantization occurs.

use crate::color::Color;
use crate::get_id;
use crate::value::ValueType;
use crate::float_image::FloatImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Rotates an image by an arbitrary angle (in degrees) around its center point.
///
/// Uses bilinear interpolation for smooth results. Areas outside the original
/// image bounds are filled with the specified background color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformRotateAroundCenter {}

impl OpImageTransformRotateAroundCenter {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate".to_string(),
            description: "Rotates an image by any angle around its center point.".to_string(),
            help: "Rotates the image by an arbitrary angle in degrees around its center, inverse-transforming each output pixel and sampling the source with bilinear interpolation directly in floating point (no 8-bit quantization). Output dimensions match the input, so corners of the rotated image are clipped and any newly uncovered pixels are filled with the background color (RGBA, so a fully transparent color leaves holes).\n\nThe output is always 4-channel RGBA; lower-channel sources are expanded the usual way (grayscale replicated to RGB, alpha preserved or set to 1). For axis-aligned quarter turns, use rotate 90 / 180 / 270 instead to avoid the resample blur.".to_string(),
        }
    }

    /// Creates the default inputs: source image, rotation angle in degrees, and background fill color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to rotate around its center."),
            Input::new("degrees".to_string(), Value::Decimal(45.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.01), clamp_to_range:false }), None)
                .with_description("Rotation angle in degrees applied around the image center."),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0,0,0,0)), None, None)
                .with_description("Color used to fill areas exposed outside the rotated image."),
        ]
    }

    /// Creates the default outputs: the rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image rotated around its center using bilinear interpolation."),
        ]
    }

    /// Executes the rotation by inverse-mapping each output pixel back into the
    /// source and bilinear-sampling the f32 data directly.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let degrees_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let bg_color_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(degrees) = degrees_converted.unwrap() else { unreachable!() };
        let Value::Color(bg_color) = bg_color_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        // Background in sRGB floats (RGBA), matching the previous RGBA fill.
        let (bg_r, bg_g, bg_b, bg_a) = bg_color.to_srgb_float();
        let bg = [bg_r, bg_g, bg_b, bg_a];

        // Precompute the inverse rotation once. Sampling at cx + dx*c + dy*s,
        // cy - dx*s + dy*c undoes a rotation by `degrees` about the center.
        let (s, c) = degrees.to_radians().sin_cos();
        let cx = (w as f32 - 1.0) / 2.0;
        let cy = (h as f32 - 1.0) / 2.0;
        let wm1 = (w.max(1) - 1) as f32;
        let hm1 = (h.max(1) - 1) as f32;
        let src = &*data;

        // Output is always 4-channel RGBA (as with the previous imageproc
        // path); lower channel counts expand like `to_rgba8` but in f32.
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let dy = y as f32 - cy;
            (0..w).flat_map(move |x| {
                let dx = x as f32 - cx;
                let sx = cx + dx * c + dy * s;
                let sy = cy - dx * s + dy * c;
                if sx >= 0.0 && sx <= wm1 && sy >= 0.0 && sy <= hm1 {
                    let mut sp = [0.0f32; 4];
                    src.bilinear_sample(sx, sy, &mut sp);
                    match ch {
                        1 => [sp[0], sp[0], sp[0], 1.0],
                        2 => [sp[0], sp[0], sp[0], sp[1]],
                        3 => [sp[0], sp[1], sp[2], 1.0],
                        _ => sp,
                    }
                } else {
                    bg
                }
            })
        }).collect();

        let output = FloatImage::from_raw(w, h, 4, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data: Arc::new(output), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "rotate_around_center_tests.rs"]
mod tests;
