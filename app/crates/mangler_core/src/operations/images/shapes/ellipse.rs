//! Ellipse shape image generator.
//!
//! Generates an anti-aliased ellipse as a grayscale SDF image with configurable
//! radii and rotation. Outputs a single-channel FloatImage mask with values
//! in [0.0, 1.0].

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
            help: "Rasterises an ellipse into a 1-channel FloatImage by evaluating a signed distance function and applying smoothstep anti-aliasing at a one-and-a-half-pixel edge width. Output is 1.0 inside and 0.0 outside.\n\nradius_x and radius_y are independent normalised axes (1.0 covers half the canvas), so setting them equal produces a circle. Rotation is applied to the normalised sample coordinates, meaning the rotated ellipse always stays centred on the image. Handy as an alpha matte or mask for blend nodes.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, radius_x, radius_y, and rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("radius_x".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal radius of the ellipse in normalised units."),
            Input::new("radius_y".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical radius of the ellipse in normalised units."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation of the ellipse around its center in degrees."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Grayscale mask with the ellipse filled white on a black background."),
        ]
    }

    /// Generates an anti-aliased ellipse image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the ellipse and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(mut width) = 0,
            Integer(mut height) = 1,
            Decimal(radius_x) = 2,
            Decimal(radius_y) = 3,
            Decimal(rotation) = 4,
        }

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
        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            // normalize to [-1, 1]
            let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
            (0..width).map(move |x| {
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

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
                let alpha = 1.0 - smoothstep_f64(-pixel_size, pixel_size, world_dist);
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
#[path = "ellipse_tests.rs"]
mod tests;
