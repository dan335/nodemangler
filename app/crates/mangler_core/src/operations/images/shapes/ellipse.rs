//! Ellipse shape image generator.
//!
//! Generates an anti-aliased ellipse as a grayscale SDF image with configurable
//! radii and rotation. Outputs a single-channel FloatImage mask with values
//! in [0.0, 1.0].

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

/// Hermite interpolation between two edges, producing a smooth transition.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Operation that generates an ellipse shape as a grayscale SDF image.
///
/// The ellipse is defined by independent X and Y radii and can be rotated.
/// Anti-aliasing is applied at the edges using a smoothstep function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeEllipse {}

impl OpImageShapeEllipse {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ellipse".to_string(),
            description: "Generates an ellipse shape as a grayscale SDF.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, radius_x, radius_y, and rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("radius_x".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("radius_y".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates an anti-aliased ellipse image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the ellipse and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let radius_x_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let radius_y_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius_x) = radius_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius_y) = radius_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);

        let rx = (radius_x as f64).max(0.001);
        let ry = (radius_y as f64).max(0.001);
        let angle = (rotation as f64).to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        // anti-aliasing width in normalized coordinates
        let pixel_size = 1.5 / (width.max(height) as f64 * 0.5);

        // 1-channel grayscale mask
        let mut image = FloatImage::new(width as u32, height as u32, 1);

        for y in 0..height {
            for x in 0..width {
                // normalize to [-1, 1]
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

                // apply rotation
                let px = nx * cos_a + ny * sin_a;
                let py = -nx * sin_a + ny * cos_a;

                // Ellipse SDF: scale coordinates by radii, then compute circular distance
                let ex = px / rx;
                let ey = py / ry;
                let dist = (ex * ex + ey * ey).sqrt() - 1.0;
                // Scale distance back to world space so anti-aliasing width is consistent
                let grad_len = ((ex / rx).powi(2) + (ey / ry).powi(2)).sqrt();
                let world_dist = if grad_len > 0.0 { dist / grad_len } else { dist };

                // smoothstep for anti-aliased edge, result in [0.0, 1.0]
                let alpha = 1.0 - smoothstep(-pixel_size, pixel_size, world_dist);
                image.put_pixel(x as u32, y as u32, &[alpha as f32]);
            }
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "ellipse_tests.rs"]
mod tests;
