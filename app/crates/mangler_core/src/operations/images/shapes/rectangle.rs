//! Rectangle shape image generator.
//!
//! Generates an anti-aliased rounded rectangle as a grayscale SDF image with
//! configurable dimensions, corner radius, and rotation. Outputs a single-channel
//! FloatImage mask with values in [0.0, 1.0].

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use crate::operations::images::adjustments::common::smoothstep_f64;

/// Operation that generates a rounded rectangle shape as a grayscale SDF image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeRectangle {}

impl OpImageShapeRectangle {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rectangle".to_string(),
            description: "Generates a rectangle shape as a grayscale SDF.".to_string(),
            help: "Rasterises a rounded rectangle into a 1-channel FloatImage using a rounded-box SDF with smoothstepped edges. rect_width and rect_height are full widths in normalised (-1..1) units — a value of 1.0 covers half the canvas.\n\ncorner_radius rounds the corners and is automatically capped at half the smaller side, so extreme values degrade to a capsule or disc rather than folding. Rotation is applied to the sample coordinates, keeping the rectangle centred on the canvas.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, rect_width, rect_height, corner_radius, and rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("rect_width".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Full width of the rectangle in normalised units."),
            Input::new("rect_height".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Full height of the rectangle in normalised units."),
            Input::new("corner_radius".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: false }), None)
                .with_description("Rounds the rectangle corners; 0 is a sharp corner."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation of the rectangle around its center in degrees."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale mask with the rectangle filled white on a black background."),
        ]
    }

    /// Generates an anti-aliased rounded rectangle image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the shape and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(mut width) = 0,
            Integer(mut height) = 1,
            Decimal(rect_width) = 2,
            Decimal(rect_height) = 3,
            Decimal(corner_radius) = 4,
            Decimal(rotation) = 5,
        }

        // run node
        width = width.max(1);
        height = height.max(1);

        let half_w = (rect_width as f64) * 0.5;
        let half_h = (rect_height as f64) * 0.5;
        let r = (corner_radius as f64).min(half_w.min(half_h));
        let angle = (rotation as f64).to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        // anti-aliasing width in normalized coordinates
        let pixel_size = 1.5 / (width.max(height) as f64 * 0.5);

        // 1-channel grayscale mask
        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            // normalize to [-1, 1]
            let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
            (0..width).map(move |x| {
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

                // apply rotation
                let px = nx * cos_a + ny * sin_a;
                let py = -nx * sin_a + ny * cos_a;

                // rounded box SDF
                let dx = px.abs() - half_w + r;
                let dy = py.abs() - half_h + r;
                let dist = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r;

                // smoothstep for anti-aliased edge, result in [0.0, 1.0]
                let alpha = 1.0 - smoothstep_f64(-pixel_size, pixel_size, dist);
                alpha as f32
            })
        }).collect();

        let image = FloatImage::from_raw(width as u32, height as u32, 1, pixels).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "rectangle_tests.rs"]
mod tests;
