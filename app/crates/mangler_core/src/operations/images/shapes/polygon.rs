//! Regular polygon shape image generator.
//!
//! Generates an anti-aliased regular polygon as a grayscale SDF image with
//! configurable side count, radius, and rotation. Outputs a single-channel
//! FloatImage mask with values in [0.0, 1.0].

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_output};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use crate::operations::images::adjustments::common::smoothstep_f64;

/// Signed distance function for a regular polygon with `n` sides and given `radius`.
fn sdf_polygon(px: f64, py: f64, radius: f64, n: i32) -> f64 {
    let n = n as f64;
    let angle_step = std::f64::consts::TAU / n;
    let half_step = angle_step * 0.5;

    let a = py.atan2(px);
    // wrap angle into one sector
    let sector_angle = ((a % angle_step) + angle_step) % angle_step - half_step;

    let r = (px * px + py * py).sqrt();
    let sx = r * sector_angle.cos();
    let sy = r * sector_angle.sin();

    // distance to the edge of the polygon in this sector
    let edge_dist = sx - radius * half_step.cos();
    let corner_y = sy.abs() - radius * half_step.sin();

    if corner_y > 0.0 {
        (edge_dist * edge_dist + corner_y * corner_y).sqrt()
    } else {
        edge_dist
    }
}

/// Operation that generates a regular polygon shape as a grayscale SDF image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapePolygon {}

impl OpImageShapePolygon {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "polygon".to_string(),
            description: "Generates a regular polygon shape as a grayscale SDF.".to_string(),
            help: "Evaluates a signed distance field for a regular polygon with N sides (minimum 3, max 64) inscribed in a circle of the given radius, then smoothsteps the boundary for anti-aliasing. Output is a 1-channel FloatImage mask with 1.0 inside and 0.0 outside.\n\nradius is the circumscribed radius in normalised (-1..1) coordinates, so the value 1.0 reaches the edge of the canvas. Rotation is applied before the SDF evaluation, which keeps the polygon centred on the image.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, sides, radius, and rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("sides".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((3.0, 64.0)), speed: None }), None)
                .with_description("Number of sides in the regular polygon (minimum 3)."),
            Input::new("radius".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Circumscribed radius of the polygon in normalised units."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation of the polygon around its center in degrees."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Grayscale mask with the polygon filled white on a black background."),
        ]
    }

    /// Generates an anti-aliased regular polygon image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the polygon and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(mut width) = 0,
            Integer(mut height) = 1,
            Integer(mut sides) = 2,
            Decimal(radius) = 3,
            Decimal(rotation) = 4,
        }

        // run node
        width = width.max(1);
        height = height.max(1);
        sides = sides.max(3);

        let rad = (radius as f64).max(0.001);
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

                let dist = sdf_polygon(px, py, rad, sides);

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
#[path = "polygon_tests.rs"]
mod tests;
