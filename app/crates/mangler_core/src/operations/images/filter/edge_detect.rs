//! Edge detection operation for images using the Sobel operator.
//!
//! Computes horizontal and vertical gradients using 3x3 Sobel kernels on
//! the Rec. 709 luminance of each pixel, then outputs the gradient magnitude
//! as a grayscale image.

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

/// Edge detection operation using Sobel gradient magnitude on luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentEdgeDetect {}

impl OpImageAdjustmentEdgeDetect {
    /// Returns the node metadata (name and description) for the edge detect operation.
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "edge detect".to_string(), description: "Detects edges using Sobel operator.".to_string() }
    }

    /// Creates the input ports: an image and an intensity multiplier for edge strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: grayscale edge magnitude image.
    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)]
    }

    /// Executes edge detection using Sobel Gx and Gy kernels on Rec. 709 luminance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node — work directly on FloatImage pixels
        let (width, height) = (data.width(), data.height());
        let mut output = (*data).clone();
        let ch = data.channels() as usize;

        // Helper: compute luminance from a pixel
        let lum_at = |px: u32, py: u32| -> f32 {
            let p = data.get_pixel(px.clamp(0, width - 1), py.clamp(0, height - 1));
            if ch >= 3 { 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2] } else { p[0] }
        };

        for y in 0..height {
            for x in 0..width {
                let x0 = if x > 0 { x - 1 } else { 0 };
                let x2 = if x + 1 < width { x + 1 } else { width - 1 };
                let y0 = if y > 0 { y - 1 } else { 0 };
                let y2 = if y + 1 < height { y + 1 } else { height - 1 };

                // Sobel Gx kernel
                let gx = -lum_at(x0, y0) - 2.0 * lum_at(x0, y) - lum_at(x0, y2)
                        + lum_at(x2, y0) + 2.0 * lum_at(x2, y) + lum_at(x2, y2);
                // Sobel Gy kernel
                let gy = -lum_at(x0, y0) - 2.0 * lum_at(x, y0) - lum_at(x2, y0)
                        + lum_at(x0, y2) + 2.0 * lum_at(x, y2) + lum_at(x2, y2);

                let magnitude = ((gx * gx + gy * gy).sqrt() * intensity).clamp(0.0, 1.0);

                let pixel = output.get_pixel_mut(x, y);
                let alpha = if ch == 2 || ch == 4 { pixel[ch - 1] } else { 1.0 };
                // Write grayscale magnitude to all color channels
                for c in 0..pixel.len().min(3) {
                    pixel[c] = magnitude;
                }
                if ch == 2 || ch == 4 { pixel[ch - 1] = alpha; }
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "edge_detect_tests.rs"]
mod tests;
