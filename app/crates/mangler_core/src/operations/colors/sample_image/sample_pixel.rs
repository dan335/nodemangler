//! Sample a single pixel color from an image.
//!
//! Reads the color at a normalized (x, y) position using bilinear
//! interpolation and emits both the combined color and its individual red,
//! green, blue, and alpha components. Lives under `colors` because its
//! headline output is a Color.

use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that samples a single pixel color from an image at a normalized position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorSampleSamplePixel {}

impl OpColorSampleSamplePixel {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sample pixel".to_string(),
            description: "Samples the color at a normalized (x, y) position in an image.".to_string(),
            help: "Reads the color at a normalized position, where x and y run from 0 (left/top) to 1 (right/bottom), using bilinear interpolation between the four nearest pixels. Emits the combined color plus its red, green, blue, and alpha components.\n\nSingle-channel images are broadcast to gray; images without an alpha channel report alpha as 1. Positions are clamped to the [0, 1] range, so sampling outside the image returns its edge pixels.".to_string(),
        }
    }

    /// Creates the input ports: the image and normalized x/y sample position.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image to sample a pixel color from."),
            Input::new("x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Horizontal position, 0 (left) to 1 (right)."),
            Input::new("y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Vertical position, 0 (top) to 1 (bottom)."),
        ]
    }

    /// Creates the output ports: the sampled color and its RGBA components.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("color".to_string(), Value::Color(Color::default()), None)
                .with_description("Sampled color at the given position."),
            Output::new("red".to_string(), Value::Decimal(0.0), None)
                .with_description("Red channel of the sampled pixel."),
            Output::new("green".to_string(), Value::Decimal(0.0), None)
                .with_description("Green channel of the sampled pixel."),
            Output::new("blue".to_string(), Value::Decimal(0.0), None)
                .with_description("Blue channel of the sampled pixel."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha of the sampled pixel (1.0 when the image has no alpha channel)."),
        ]
    }

    /// Executes the pixel-sampling operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;

        let px = x.clamp(0.0, 1.0) * (w.saturating_sub(1) as f32);
        let py = y.clamp(0.0, 1.0) * (h.saturating_sub(1) as f32);

        let mut buf = [0.0f32; 4];
        data.bilinear_sample(px, py, &mut buf[..ch]);

        let (r, g, b, a) = match ch {
            1 => (buf[0], buf[0], buf[0], 1.0),
            2 => (buf[0], buf[0], buf[0], buf[1]),
            3 => (buf[0], buf[1], buf[2], 1.0),
            _ => (buf[0], buf[1], buf[2], buf[3]),
        };
        let color = Color::from_srgb_float(r, g, b, a);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(color) },
                OutputResponse { value: Value::Decimal(r) },
                OutputResponse { value: Value::Decimal(g) },
                OutputResponse { value: Value::Decimal(b) },
                OutputResponse { value: Value::Decimal(a) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "sample_pixel_tests.rs"]
mod tests;
