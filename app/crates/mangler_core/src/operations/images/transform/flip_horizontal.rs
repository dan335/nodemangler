//! Horizontal flip (mirror left-to-right) operation.
//!
//! Operates directly on [`FloatImage`] pixel data.

use crate::get_id;
use crate::float_image::FloatImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Flips an image horizontally (mirrors left-to-right).
///
/// Applying this operation twice restores the original image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformFlipHorizontal {}

impl OpImageTransformFlipHorizontal {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flip horizontal".to_string(),
            description: "Flips an image horizontally.".to_string(),
            help: "Mirrors every pixel across the vertical center axis, so the pixel at (x, y) becomes (width - 1 - x, y). Output dimensions and channel count are identical to the input; no resampling or interpolation is performed, which means this is lossless and applying the operation twice restores the original image exactly.".to_string(),
        }
    }

    /// Creates the default inputs: a single source image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to flip left-to-right."),
        ]
    }

    /// Creates the default outputs: the flipped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image mirrored left-to-right."),
        ]
    }

    /// Executes the horizontal flip by mirroring pixels left-to-right.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
        }

        // Create output with same dimensions
        let (w, h) = data.dimensions();
        let mut output = FloatImage::new(w, h, data.channels());

        // Mirror pixels: source(x, y) -> output(w-1-x, y)
        for y in 0..h {
            for x in 0..w {
                output.put_pixel(w - 1 - x, y, data.get_pixel(x, y));
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
#[path = "flip_horizontal_tests.rs"]
mod tests;
