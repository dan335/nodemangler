//! Image dimensions operation.
//!
//! Reads an image and reports its width, height, aspect ratio, and channel
//! count as numbers, so downstream math can be driven by an existing image's
//! own size (e.g. resizing or tiling relative to a loaded file).

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports an image's width, height, aspect ratio, and channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageDimensions {}

impl OpNumberImageDimensions {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "dimensions".to_string(),
            description: "Reports an image's width, height, aspect ratio, and channel count.".to_string(),
            help: "Reads the source image and outputs its pixel width and height as integers, its aspect ratio (width / height) as a decimal, and its channel count (1–4) as an integer.\n\nUse it to drive math off an image's own size — for example feeding width and height into a resize, crop, or tiling node so the graph adapts to whatever image is loaded rather than to hard-coded numbers. This is a pure measurement: the image itself is not modified.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose dimensions are measured."),
        ]
    }

    /// Creates the output ports: width, height, aspect ratio, channels.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Image width in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Image height in pixels."),
            Output::new("aspect ratio".to_string(), Value::Decimal(1.0), None)
                .with_description("Width divided by height (1.0 for a square image)."),
            Output::new("channels".to_string(), Value::Integer(4), None)
                .with_description("Number of channels per pixel (1–4)."),
        ]
    }

    /// Executes the measurement.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let aspect = if h > 0 { w as f32 / h as f32 } else { 0.0 };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Integer(w as i32) },
                OutputResponse { value: Value::Integer(h as i32) },
                OutputResponse { value: Value::Decimal(aspect) },
                OutputResponse { value: Value::Integer(data.channels() as i32) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "dimensions_tests.rs"]
mod tests;
