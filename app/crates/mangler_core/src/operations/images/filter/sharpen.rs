//! Convolution-based sharpening operation for images.
//!
//! Applies a 3x3 sharpening kernel where the center weight is boosted and
//! edge weights are negative, enhancing local contrast at edges.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Convolution-based sharpening operation using a 3x3 edge-enhancement kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSharpen {}

impl OpImageAdjustmentSharpen {
    /// Returns the node metadata (name and description) for the sharpen operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sharpen".to_string(),
            description: "Sharpens an image using a convolution kernel.".to_string(),
        }
    }

    /// Creates the input ports: an image and an intensity controlling sharpening strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the sharpened image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the sharpening convolution. Uses edge-clamped sampling for border pixels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node — work directly on FloatImage pixels
        let (width, height) = (data.width(), data.height());
        let mut output = (*data).clone();
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Sharpen kernel: center = 1 + 4*intensity, edges = -intensity, corners = 0
        let center = 1.0 + 4.0 * intensity;
        let edge = -intensity;

        for y in 0..height {
            for x in 0..width {
                let x0 = if x > 0 { x - 1 } else { 0 };
                let x2 = if x + 1 < width { x + 1 } else { width - 1 };
                let y0 = if y > 0 { y - 1 } else { 0 };
                let y2 = if y + 1 < height { y + 1 } else { height - 1 };

                let c_val = data.get_pixel(x, y);
                let top = data.get_pixel(x, y0);
                let bottom = data.get_pixel(x, y2);
                let left = data.get_pixel(x0, y);
                let right = data.get_pixel(x2, y);

                let pixel = output.get_pixel_mut(x, y);
                for c in 0..color_ch {
                    let val = center * c_val[c]
                        + edge * top[c]
                        + edge * bottom[c]
                        + edge * left[c]
                        + edge * right[c];
                    pixel[c] = val.clamp(0.0, 1.0);
                }
                // alpha unchanged
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
#[path = "sharpen_tests.rs"]
mod tests;
