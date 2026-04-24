//! 270-degree clockwise (90-degree counter-clockwise) rotation operation.
//!
//! Operates directly on [`FloatImage`] pixel data, swapping dimensions.

use crate::get_id;
use crate::float_image::FloatImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Rotates an image 270 degrees clockwise (equivalent to 90 degrees counter-clockwise).
///
/// The output dimensions are swapped: width becomes height and vice versa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformRotate270 {}

impl OpImageTransformRotate270 {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate 270".to_string(),
            description: "Rotates an image 270 degrees.".to_string(),
            help: "Rotates the image 270 degrees clockwise, equivalent to 90 degrees counter-clockwise. Pixel (x, y) in the source maps to (y, width - 1 - x) in the output, and the output's width and height are swapped relative to the input.\n\nNo resampling is performed, so the operation is lossless and the channel count is preserved. Use this or `rotate 90` / `rotate 180` for axis-aligned rotations to avoid the bicubic blur introduced by the arbitrary-angle `rotate` node.".to_string(),
        }
    }

    /// Creates the default inputs: a single source image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to rotate 270 degrees clockwise."),
        ]
    }

    /// Creates the default outputs: the rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image rotated 270 degrees clockwise with width and height swapped."),
        ]
    }

    /// Executes the 270-degree clockwise rotation by rearranging pixels.
    ///
    /// For a 270-degree clockwise rotation, pixel at (x, y) in the source
    /// maps to (y, w - 1 - x) in the output (with swapped dimensions).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // Create output with swapped dimensions
        let (w, h) = data.dimensions();
        let mut output = FloatImage::new(h, w, data.channels());

        // Rearrange pixels: source(x, y) -> output(y, w-1-x)
        for y in 0..h {
            for x in 0..w {
                output.put_pixel(y, w - 1 - x, data.get_pixel(x, y));
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data: Arc::new(output), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "rotate_270_tests.rs"]
mod tests;
