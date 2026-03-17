use image::{ImageBuffer, DynamicImage};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternWeave {}

impl OpImagePatternWeave {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "weave".to_string(),
            description: "Generates a basket weave pattern.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("count".to_string(), Value::Integer(8), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None),
            Input::new("gap_size".to_string(), Value::Decimal(0.05), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let count_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let gap_size_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut count) = count_converted.unwrap() else { unreachable!() };
        let Value::Decimal(gap_size) = gap_size_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        count = count.max(1);
        let gap_size = (gap_size as f64).clamp(0.0, 0.5);

        let cell_width = width as f64 / count as f64;
        let cell_height = height as f64 / count as f64;

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        for py in 0..height {
            for px in 0..width {
                let col = (px as f64 / cell_width).floor() as i32;
                let row = (py as f64 / cell_height).floor() as i32;

                let x_in_cell = (px as f64 % cell_width) / cell_width;
                let y_in_cell = (py as f64 % cell_height) / cell_height;

                // check if pixel is in the gap area
                let in_gap = x_in_cell < gap_size || x_in_cell > (1.0 - gap_size)
                    || y_in_cell < gap_size || y_in_cell > (1.0 - gap_size);

                let g: u8 = if in_gap {
                    0
                } else {
                    // checkerboard pattern: alternating horizontal and vertical strands
                    let is_horizontal = (col + row) % 2 == 0;
                    if is_horizontal { 200 } else { 128 }
                };

                image_buffer.put_pixel(px as u32, py as u32, image::Luma([g]));
            }
        }

        let dynamic_image = DynamicImage::ImageLuma8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
            ],
        })
    }
}
