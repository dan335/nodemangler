//! Line shape image generator.
//!
//! Generates an anti-aliased line segment as a grayscale SDF image with
//! configurable start/end points and thickness. Outputs a single-channel
//! FloatImage mask with values in [0.0, 1.0].

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Hermite interpolation between two edges, producing a smooth transition.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

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
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale mask with the line drawn white on a black background."),
        ]
    }

    /// Generates an anti-aliased line segment image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the line and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let start_x_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let start_y_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let end_x_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let end_y_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let thickness_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(start_x) = start_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(start_y) = start_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(end_x) = end_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(end_y) = end_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness) = thickness_converted.unwrap() else { unreachable!() };

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
        // anti-aliasing width in normalized coordinates
        let pixel_size = 1.5 / (width.max(height) as f64 * 0.5);

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
                let alpha = 1.0 - smoothstep(-pixel_size, pixel_size, dist);
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
