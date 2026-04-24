//! Drop shadow from a mask.
//!
//! Offsets the mask by `(offset_x, offset_y)` pixels, blurs it, and outputs
//! an RGBA image coloured by `color` with the blurred mask as alpha times
//! `opacity`. Composite below the source using a `blend` node in Normal mode.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Drop shadow: offset + blurred mask as a tinted alpha layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageFxDropShadow {}

impl OpImageFxDropShadow {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "drop shadow".to_string(),
            description: "Offsets and blurs a mask, tints it, and outputs an RGBA shadow layer for compositing below the source.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("offset x".to_string(), Value::Decimal(6.0), Some(InputSettings::DragValue { speed: None, clamp: Some((-256.0, 256.0)) }), None),
            Input::new("offset y".to_string(), Value::Decimal(6.0), Some(InputSettings::DragValue { speed: None, clamp: Some((-256.0, 256.0)) }), None),
            Input::new("blur radius".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 256.0)) }), None),
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("opacity".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let mask_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let off_x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let off_y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let blur_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let color_converted = convert_input(inputs, 4, ValueType::Color, &mut input_errors);
        let opacity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = mask_converted.unwrap() else { unreachable!() };
        let Value::Decimal(off_x) = off_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(off_y) = off_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blur) = blur_converted.unwrap() else { unreachable!() };
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(opacity) = opacity_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Pull the mask's alpha (or luminance for single/triple-channel) into a scalar field.
        let mut mask_field = FloatImage::new(width, height, 1);
        for y in 0..height {
            for x in 0..width {
                let p = data.get_pixel(x, y);
                let m = match ch {
                    1 => p[0],
                    2 => p[0] * p[1],
                    3 => 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2],
                    _ => (0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]) * p[3],
                };
                mask_field.put_pixel(x, y, &[m]);
            }
        }

        // Offset the mask field. We do a separate offset + blur pass rather
        // than folding them together so the caller can use zero blur for a
        // crisp offset shadow.
        let ox = off_x.round() as i32;
        let oy = off_y.round() as i32;
        let mut offset_field = FloatImage::new(width, height, 1);
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let sx = x - ox;
                let sy = y - oy;
                let v = if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                    0.0
                } else {
                    mask_field.get_pixel(sx as u32, sy as u32)[0]
                };
                offset_field.put_pixel(x as u32, y as u32, &[v]);
            }
        }

        let blurred = gaussian_blur_image(&offset_field, blur.max(0.0));

        let (cr, cg, cb, ca) = color.to_srgb_float();
        let mut output = FloatImage::new(width, height, 4);
        let opacity = opacity.clamp(0.0, 1.0);
        for y in 0..height {
            for x in 0..width {
                let a = blurred.get_pixel(x, y)[0] * ca * opacity;
                output.put_pixel(x, y, &[cr, cg, cb, a.clamp(0.0, 1.0)]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "drop_shadow_tests.rs"]
mod tests;
