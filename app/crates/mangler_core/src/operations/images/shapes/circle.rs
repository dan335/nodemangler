//! Circle shape image generator.
//!
//! Generates an anti-aliased filled circle as a grayscale SDF image with a
//! configurable radius and center. Unlike the `ellipse` node (which scales its
//! X and Y axes independently in normalized space), the circle is measured in
//! pixel space, so it stays perfectly round on non-square canvases. Outputs a
//! single-channel FloatImage mask with values in [0.0, 1.0].

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

/// Operation that generates a filled circle as a grayscale SDF image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapesCircle {}

impl OpImageShapesCircle {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "circle".to_string(),
            description: "Generates a filled circle as a grayscale SDF.".to_string(),
            help: "Rasterises a filled circle into a 1-channel FloatImage by evaluating a signed distance function in pixel space and applying smoothstep anti-aliasing at a one-and-a-half-pixel edge width. Output is 1.0 inside the disc and 0.0 outside.\n\nradius is normalised so that 1.0 spans half of the shorter image dimension, which keeps the shape perfectly round even on non-square canvases (the ellipse node, by contrast, normalises each axis independently). center_x and center_y offset the disc from the middle in units of half the canvas (0 = centred, 1 = the corresponding edge). Handy as an alpha matte or mask for blend nodes.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, radius, center_x, and center_y.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("radius".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Circle radius; 1.0 spans half the shorter image dimension."),
            Input::new("center_x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center offset from the middle, in units of half the canvas width."),
            Input::new("center_y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center offset from the middle, in units of half the canvas height."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale mask with the circle filled white on a black background."),
        ]
    }

    /// Generates an anti-aliased filled-circle image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the circle and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let center_x_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let center_y_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(center_x) = center_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(center_y) = center_y_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);

        // Work in pixel space so the circle stays round regardless of aspect.
        let min_dim = width.min(height) as f64;
        let radius_px = (radius as f64).max(0.0) * min_dim * 0.5;
        let center_px_x = width as f64 * 0.5 * (1.0 + center_x as f64);
        let center_px_y = height as f64 * 0.5 * (1.0 + center_y as f64);
        // anti-aliasing half-width, in pixels
        let aa = 1.5;

        // 1-channel grayscale mask
        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            // Sample at the pixel center.
            let dy = (y as f64 + 0.5) - center_px_y;
            (0..width).map(move |x| {
                let dx = (x as f64 + 0.5) - center_px_x;
                // Signed distance to the circle edge (negative inside).
                let dist = (dx * dx + dy * dy).sqrt() - radius_px;
                // smoothstep for anti-aliased edge, result in [0.0, 1.0]
                let alpha = 1.0 - smoothstep(-aa, aa, dist);
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
#[path = "circle_tests.rs"]
mod tests;
