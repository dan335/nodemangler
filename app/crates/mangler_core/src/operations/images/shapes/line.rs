//! Line shape image generator.
//!
//! Generates an anti-aliased line segment as a grayscale SDF image with
//! configurable start/end points and thickness. Outputs a single-channel
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

/// Operation that generates a line segment as a grayscale SDF image.
///
/// The line is defined by start and end points in normalized `[0, 1]` coordinates
/// and a thickness value. Handles the degenerate case where start equals end
/// by rendering a circle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeLine {}

impl OpImageShapeLine {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "line".to_string(),
            description: "Generates a line shape as a grayscale SDF.".to_string(),
            help: "Draws an anti-aliased line segment from (start_x, start_y) to (end_x, end_y) with rounded caps, by evaluating the signed distance to the segment and smoothstepping the edge. Coordinates are 0-1 fractions of the canvas; thickness is in normalised units.\n\nOutput is a 1-channel FloatImage mask with 1.0 inside the stroke and 0.0 outside. When start and end coincide the node degrades gracefully into a filled disc centred on that point rather than producing an empty image.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, start_x, start_y, end_x, end_y, and thickness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("start_x".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Start-point X as a 0-1 fraction of image width."),
            Input::new("start_y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Start-point Y as a 0-1 fraction of image height."),
            Input::new("end_x".to_string(), Value::Decimal(0.75), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("End-point X as a 0-1 fraction of image width."),
            Input::new("end_y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("End-point Y as a 0-1 fraction of image height."),
            Input::new("thickness".to_string(), Value::Decimal(0.02), Some(InputSettings::Slider { range: (0.001, 0.2), step_by: None, clamp_to_range: false }), None)
                .with_description("Thickness of the line in normalised units."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Grayscale mask with the line drawn white on a black background."),
        ]
    }

    /// Generates an anti-aliased line segment image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the line and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(mut width) = 0,
            Integer(mut height) = 1,
            Decimal(start_x) = 2,
            Decimal(start_y) = 3,
            Decimal(end_x) = 4,
            Decimal(end_y) = 5,
            Decimal(thickness) = 6,
        }

        // run node
        width = width.max(1);
        height = height.max(1);

        let half_thick = (thickness as f64).max(0.0001) * 0.5;
        // convert start/end from [0,1] to [-1,1]
        let ax = (start_x as f64) * 2.0 - 1.0;
        let ay = (start_y as f64) * 2.0 - 1.0;
        let bx = (end_x as f64) * 2.0 - 1.0;
        let by = (end_y as f64) * 2.0 - 1.0;

        let dx = bx - ax;
        let dy = by - ay;
        let seg_len_sq = dx * dx + dy * dy;
        // Anti-aliasing half-width in normalized coordinates. `dist` is a true
        // distance in the [-1,1] space where one pixel spans 2/max_dim, so a
        // half-width of 1/max_dim ramps the edge over ~1 pixel — crisp. (The
        // old 3/max_dim value spread the ramp over ~3 pixels, which looked blurry.)
        let pixel_size = 1.0 / (width.max(height) as f64);

        // 1-channel grayscale mask
        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            // normalize to [-1, 1]
            let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
            (0..width).map(move |x| {
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

                // line segment SDF
                let dist = if seg_len_sq < 1e-12 {
                    // degenerate line (point) — renders as a circle
                    ((nx - ax).powi(2) + (ny - ay).powi(2)).sqrt() - half_thick
                } else {
                    let t = ((nx - ax) * dx + (ny - ay) * dy) / seg_len_sq;
                    let t = t.clamp(0.0, 1.0);
                    let cx = ax + t * dx;
                    let cy = ay + t * dy;
                    ((nx - cx).powi(2) + (ny - cy).powi(2)).sqrt() - half_thick
                };

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
#[path = "line_tests.rs"]
mod tests;
