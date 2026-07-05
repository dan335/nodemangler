//! Constant grayscale image generation operation.
//!
//! Creates a single-channel `FloatImage` of a specified width and height where
//! every pixel is filled with the same scalar value. This is the number → image
//! bridge, useful as a base layer, mask, or height map.

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

/// Operation that generates a uniform single-channel grayscale image.
///
/// Accepts a scalar value, width, and height, and produces a 1-channel image
/// where every pixel is set to the given value. Also passes through the value
/// and dimensions as separate outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputConstant {}

impl OpImageInputConstant {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "constant".to_string(),
            description: "Creates a solid grayscale image from a number.".to_string(),
            help: "Produces a 1-channel grayscale FloatImage where every pixel holds the same scalar value in [0, 1]. Width and height are clamped to at least 1 and capped at 10000.\n\nThis is the number → image bridge: handy as a base layer, a constant mask, or a flat height map for the PBR and simulation nodes. Contrast with `from color`, which fills a 4-channel RGBA image from a color rather than a single value. The value, width, and height are also passed through as separate outputs.".to_string(),
        }
    }

    /// Creates the input definitions: value (0-1 slider), width (1-10000), and height (1-10000).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("value".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Scalar value written to every pixel of the output image."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
        ]
    }

    /// Creates the output definitions: the generated image, the value, width, and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Solid grayscale image filled with the chosen value."),
            Output::new("value".to_string(), Value::Decimal(0.5), None)
                .with_description("Pass-through of the input value."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Final width of the generated image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Final height of the generated image in pixels."),
        ]
    }

    /// Executes the operation: creates a 1-channel image buffer filled with the input value.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let value_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(value) = value_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };

        // run node — clamp dimensions to at least 1
        width = width.max(1);
        height = height.max(1);

        // Create a 1-channel grayscale FloatImage filled with the scalar value.
        let float_img = FloatImage::from_pixel(
            width as u32,
            height as u32,
            1,
            &[value],
        );

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_img), change_id: get_id() } },
                OutputResponse { value: Value::Decimal(value) },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "constant_tests.rs"]
mod tests;
