//! Basket weave pattern image generator.
//!
//! Generates a weave pattern where horizontal and vertical strands alternate
//! in a checkerboard fashion, separated by configurable gaps. Horizontal strands
//! are rendered brighter (~0.784) than vertical strands (~0.502) to simulate depth.
//! Outputs a single-channel FloatImage.

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

/// Operation that generates a basket weave pattern as a grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternWeave {}

impl OpImagePatternWeave {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "weave".to_string(),
            description: "Generates a basket weave pattern.".to_string(),
            help: "Produces a single-channel basket-weave mask on a count-by-count grid. Cells alternate in a checkerboard between brighter horizontal strands (~0.784) and dimmer vertical strands (~0.502) to fake depth, with gaps between strands written as 0.0.\n\nGap size is a fraction of cell size and applies to all four sides of each cell. Increasing count adds more, smaller strands. The pattern repeats cleanly on an integer number of cells so it will tile when width and height are multiples of the cell size.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, count (grid divisions), and gap_size.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("count".to_string(), Value::Integer(8), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None)
                .with_description("Grid divisions along both axes; controls strand density."),
            Input::new("gap_size".to_string(), Value::Decimal(0.05), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: true }), None)
                .with_description("Gap thickness between strands as a fraction of cell size."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale basket-weave mask with brighter horizontal and dimmer vertical strands."),
        ]
    }

    /// Generates a basket weave pattern image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where horizontal strands are ~0.784,
    /// vertical strands are ~0.502, and gaps are 0.0.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let count_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let gap_size_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

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

        // brightness values matching the original u8 values (200/255, 128/255)
        let horizontal_brightness: f32 = 200.0 / 255.0;
        let vertical_brightness: f32 = 128.0 / 255.0;

        // 1-channel grayscale mask
        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |py| {
            let row = (py as f64 / cell_height).floor() as i32;
            let y_in_cell = (py as f64 % cell_height) / cell_height;

            (0..width).map(move |px| {
                let col = (px as f64 / cell_width).floor() as i32;
                let x_in_cell = (px as f64 % cell_width) / cell_width;

                // check if pixel is in the gap area
                let in_gap = x_in_cell < gap_size || x_in_cell > (1.0 - gap_size)
                    || y_in_cell < gap_size || y_in_cell > (1.0 - gap_size);

                if in_gap {
                    0.0
                } else {
                    // checkerboard pattern: alternating horizontal and vertical strands
                    let is_horizontal = (col + row) % 2 == 0;
                    if is_horizontal { horizontal_brightness } else { vertical_brightness }
                }
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
#[path = "weave_tests.rs"]
mod tests;
