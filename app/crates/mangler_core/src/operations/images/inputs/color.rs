//! Solid-color image generation operation.
//!
//! Creates an image of a specified width and height where every pixel is
//! filled with the same color. The color is stored as a 4-channel `FloatImage`
//! with sRGB float values directly, avoiding u8 quantisation.

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

/// Operation that generates a uniform solid-color image.
///
/// Accepts a color, width, and height, and produces an RGBA image where
/// every pixel is set to the given color. Also passes through the color
/// and dimensions as separate outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputColor {}

impl OpImageInputColor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from color".to_string(),
            description: "Creates an image from a color.".to_string(),
            help: "Produces a 4-channel FloatImage where every pixel is the same color. Width and height are clamped to at least 1 and capped at 10000; the color is written directly as sRGB floats to avoid the precision loss of a u8 round-trip.\n\nHandy for generating solid backgrounds, tinted fills for blend operations, or constant alpha masks. The color, width, and height are also passed through as separate outputs so downstream nodes can read the same values without re-wiring the original inputs.".to_string(),
        }
    }

    /// Creates the input definitions: color, width (1-10000), and height (1-10000).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color used to fill every pixel of the output image."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
        ]
    }

    /// Creates the output definitions: the generated image, the color, width, and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Solid-color image filled with the chosen color."),
            Output::new("color".to_string(), Value::Color(Color::default()), None)
                .with_description("Pass-through of the input color."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Final width of the generated image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Final height of the generated image in pixels."),
        ]
    }

    /// Executes the operation: creates an image buffer filled with the input color.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };

        // run node — clamp dimensions to at least 1
        width = width.max(1);
        height = height.max(1);

        // Create a 4-channel FloatImage filled with the sRGB float color directly.
        // This avoids u8 quantisation, preserving full float precision.
        let srgb = color.to_srgb_float();
        let float_img = FloatImage::from_pixel(
            width as u32,
            height as u32,
            4,
            &[srgb.0, srgb.1, srgb.2, srgb.3],
        );

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_img), change_id: get_id() } },
                OutputResponse { value: Value::Color(color) },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
