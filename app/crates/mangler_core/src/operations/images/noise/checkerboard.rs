//! Checkerboard pattern image generator.
//!
//! Produces a grayscale checkerboard pattern using the noise library's
//! `Checkerboard` function. The cell size controls the scale of the squares.

use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
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
use noise::{NoiseFn, Checkerboard};

/// Operation that generates a checkerboard pattern as a grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCheckerboard {}

impl OpImageNoiseCheckerboard {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "checkerboard noise".to_string(),
            description: "Creates a checkerboard noise image.".to_string(),
            help: "Not a stochastic noise at all: a deterministic alternating black/white grid produced by the noise crate's Checkerboard function. The size input is a subdivision exponent rather than a count, so each step doubles how many squares fit across the image.\n\nHandy as a UV test pattern, a mask for regular alternation, or an input to other nodes (warp, blur, blend) that turn the regular grid into something less obvious.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, and size (number of checkerboard divisions).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("size".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Checkerboard subdivision exponent; larger values produce more, smaller squares."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Grayscale checkerboard pattern image."),
        ]
    }

    /// Generates a checkerboard pattern image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Collect every input conversion into the shared error buffer so all
        // malformed inputs surface in a single error response, matching the
        // rest of the operation library.
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let size_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(size) = size_converted.unwrap() else { unreachable!() };

        let width = width.max(1);
        let height = height.max(1);
        let size = size.max(1);

        // Build a single-channel FloatImage from the checkerboard pattern
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);

        let perlin = Checkerboard::new(size as usize);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64);
                let coords_y = (y as f64) / (size as f64);
                let noise = perlin.get([coords_x, coords_y]) as f32 * 0.5 + 0.5;
                let non_linear = linear_to_nonlinear_srgb(noise);
                float_image.put_pixel(x as u32, y as u32, &[non_linear]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "checkerboard_tests.rs"]
mod tests;
