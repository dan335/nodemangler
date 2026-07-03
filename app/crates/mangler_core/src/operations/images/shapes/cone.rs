//! Cone height shape: circular base, linearly falling height toward the edge.
//!
//! `truncate` clips the top to create a flat-topped frustum.

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

/// Conical height shape, optionally truncated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeCone {}

impl OpImageShapeCone {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cone".to_string(),
            description: "Cone height shape. `truncate` flattens the peak to produce a frustum.".to_string(),
            help: "Produces a 1-channel greyscale height field where value equals linear falloff from the center of a circular base, reaching 1.0 at the apex and 0.0 outside the radius. size is the base radius in normalised (-1..1) coordinates and caps at 0.001 to avoid division-by-zero.\n\nRaising truncate clips the top of the cone and renormalises so the remaining plateau still peaks at 1.0, giving a frustum. Ideal as a height input for normal_from_height and ao_from_height downstream; values are linear, not sRGB.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated height map in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated height map in pixels."),
            Input::new("size".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius of the cone's circular base in normalised units."),
            Input::new("truncate".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Cuts off the top of the cone to create a flat-topped frustum."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale cone height shape, 1.0 at the peak and 0.0 outside the base."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let w_c = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let h_c = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let size_c = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let trunc_c = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut width) = w_c.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = h_c.unwrap() else { unreachable!() };
        let Value::Decimal(size) = size_c.unwrap() else { unreachable!() };
        let Value::Decimal(truncate) = trunc_c.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let size = (size as f64).max(0.001);
        let truncate = (truncate as f64).clamp(0.0, 0.99);

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
            (0..width).map(move |x| {
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let d = (nx * nx + ny * ny).sqrt() / size;
                let h = if d >= 1.0 {
                    0.0
                } else {
                    // 1-d is the raw cone; truncating lops off the top and re-normalises
                    // so the plateau still reaches 1.0.
                    let raw = (1.0 - d).min(1.0 - truncate);
                    if truncate >= 1.0 { 0.0 } else { raw / (1.0 - truncate) }
                };
                h as f32
            })
        }).collect();

        let img = FloatImage::from_raw(width as u32, height as u32, 1, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(img), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "cone_tests.rs"]
mod tests;
