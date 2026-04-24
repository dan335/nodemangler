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
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("size".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("truncate".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
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

        let mut img = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..height {
            for x in 0..width {
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let d = (nx * nx + ny * ny).sqrt() / size;
                let h = if d >= 1.0 {
                    0.0
                } else {
                    // 1-d is the raw cone; truncating lops off the top and re-normalises
                    // so the plateau still reaches 1.0.
                    let raw = (1.0 - d).min(1.0 - truncate);
                    if truncate >= 1.0 { 0.0 } else { raw / (1.0 - truncate) }
                };
                img.put_pixel(x as u32, y as u32, &[h as f32]);
            }
        }

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
