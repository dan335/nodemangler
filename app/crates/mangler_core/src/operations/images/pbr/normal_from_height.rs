//! Normal map generation from a height map.
//!
//! Computes a tangent-space normal map from a grayscale height map using the
//! Sobel operator for gradient estimation. The resulting normals are encoded
//! in the standard `[0, 1]` range where `(0.5, 0.5, 1.0)` represents a flat surface.

use crate::float_image::FloatImage;
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

/// Operation that generates a normal map from a grayscale height map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrNormalFromHeight{}

impl OpImagePbrNormalFromHeight {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "normal from height".to_string(), description: "Generates a normal map from a grayscale height map.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 20.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)]
    }

    /// Generates a normal map using the Sobel operator on luminance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let width = data.width() as i32;
        let height = data.height() as i32;
        let ch = data.channels() as usize;

        // Compute luminance (Rec. 709) of a pixel, clamping coords to image bounds
        let luminance = |x: i32, y: i32| -> f32 {
            let cx = x.clamp(0, width - 1) as u32;
            let cy = y.clamp(0, height - 1) as u32;
            let p = data.get_pixel(cx, cy);
            if ch >= 3 { 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2] } else { p[0] }
        };

        let mut buffer = FloatImage::new(width as u32, height as u32, 4);

        for y in 0..height {
            for x in 0..width {
                let tl = luminance(x - 1, y - 1);
                let top = luminance(x, y - 1);
                let tr = luminance(x + 1, y - 1);
                let left = luminance(x - 1, y);
                let right = luminance(x + 1, y);
                let bl = luminance(x - 1, y + 1);
                let bottom = luminance(x, y + 1);
                let br = luminance(x + 1, y + 1);

                // Sobel operator
                let dx = ((tr + 2.0 * right + br) - (tl + 2.0 * left + bl)) * intensity;
                let dy = ((bl + 2.0 * bottom + br) - (tl + 2.0 * top + tr)) * intensity;

                // Compute and normalize normal vector
                let nx = -dx;
                let ny = -dy;
                let nz = 1.0_f32;
                let len = (nx * nx + ny * ny + nz * nz).sqrt();

                // Map from [-1,1] to [0,1]
                let r = (nx / len) * 0.5 + 0.5;
                let g = (ny / len) * 0.5 + 0.5;
                let b = (nz / len) * 0.5 + 0.5;

                buffer.put_pixel(x as u32, y as u32, &[r, g, b, 1.0]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {value: Value::Image { data:Arc::new(buffer), change_id:get_id() }}],
        })
    }
}

#[cfg(test)]
#[path = "normal_from_height_tests.rs"]
mod tests;
