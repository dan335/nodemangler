//! Checkerboard pattern image generator.
//!
//! Produces a grayscale checkerboard pattern using the noise library's
//! `Checkerboard` function. The cell size controls the scale of the squares.

use image::{RgbaImage, ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::color::color_spaces::rgb_linear::{nonlinear_to_linear_rgb, linear_to_nonlinear_srgb};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use noise::{NoiseFn, Perlin, Seedable, Checkerboard};

/// Operation that generates a checkerboard pattern as a grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCheckerboard {}

impl OpImageNoiseCheckerboard {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "checkerboard noise".to_string(),
            description: "Creates a checkerboard noise image.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, and size (number of checkerboard divisions).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("size".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
        ]
    }

    /// Generates a checkerboard pattern image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = inputs[0].value.try_convert_to(ValueType::Integer);
        let height_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let size_converted = inputs[2].value.try_convert_to(ValueType::Integer);
        
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Integer(mut width)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to integer.".to_string())], node_error: None })};
        let Ok(Value::Integer(mut height)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert to integer.".to_string())], node_error: None })};
        let Ok(Value::Integer(mut size)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(2, "Unable to convert to integer.".to_string())], node_error: None })};
        
        width = width.max(1);
        height = height.max(1);
        size = size.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let perlin = Checkerboard::new(size as usize);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64);
                let coords_y = (y as f64) / (size as f64);
                let noise = perlin.get([coords_x, coords_y]) as f32 * 0.5 + 0.5;
                let non_linear = linear_to_nonlinear_srgb(noise);
                let g = (non_linear * 65535.0) as u16;
                image_buffer.put_pixel(x as u32, y as u32, image::Luma([g]));
            }
        }
        
        let dynamic_image = DynamicImage::ImageLuma16(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: dynamic_image, change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "checkerboard_tests.rs"]
mod tests;
