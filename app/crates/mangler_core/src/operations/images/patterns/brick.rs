//! Brick pattern image generator.
//!
//! Generates a tileable brick wall pattern as a grayscale image where 1.0
//! represents bricks and 0.0 represents mortar gaps. Supports configurable
//! row/column count, row offset (staggering), and gap size. Outputs a
//! single-channel FloatImage.

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

/// Operation that generates a brick wall pattern as a grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternBrick {}

impl OpImagePatternBrick {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "brick".to_string(),
            description: "Generates a brick pattern.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, columns, rows, offset, and gap_size.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("columns".to_string(), Value::Integer(8), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None),
            Input::new("rows".to_string(), Value::Integer(16), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None),
            Input::new("offset".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None),
            Input::new("gap_size".to_string(), Value::Decimal(0.05), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates a brick pattern image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = brick and 0.0 = mortar gap.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let columns_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let rows_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let offset_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let gap_size_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut columns) = columns_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut rows) = rows_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset) = offset_converted.unwrap() else { unreachable!() };
        let Value::Decimal(gap_size) = gap_size_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        columns = columns.max(1);
        rows = rows.max(1);
        let gap_size = gap_size.clamp(0.0, 0.5) as f64;
        let offset = offset.clamp(0.0, 1.0) as f64;

        let cell_width = width as f64 / columns as f64;
        let cell_height = height as f64 / rows as f64;

        // 1-channel grayscale mask
        let mut image = FloatImage::new(width as u32, height as u32, 1);

        for y in 0..height {
            let row = (y as f64 / cell_height).floor() as i32;
            let y_in_cell = (y as f64 % cell_height) / cell_height;

            // Stagger odd rows by the offset fraction of cell width
            let row_offset = if row % 2 != 0 { offset * cell_width } else { 0.0 };

            for x in 0..width {
                let shifted_x = (x as f64 + row_offset) % width as f64;
                let x_in_cell = (shifted_x % cell_width) / cell_width;

                // check if pixel is in the gap area
                let in_gap = x_in_cell < gap_size || x_in_cell > (1.0 - gap_size)
                    || y_in_cell < gap_size || y_in_cell > (1.0 - gap_size);

                // 1.0 for brick, 0.0 for mortar gap
                let val: f32 = if in_gap { 0.0 } else { 1.0 };
                image.put_pixel(x as u32, y as u32, &[val]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "brick_tests.rs"]
mod tests;
