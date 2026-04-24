//! Map a flood-fill data image into colour using per-cell random values.
//!
//! Reads the output of `flood_fill` and, per pixel, looks up the cell's
//! random value in a user-supplied gradient image (horizontally sampled).
//! Pixels whose cell id is 0 (outside the mask) pass through as black or
//! transparent.
//!
//! Gradient sampling: the random value maps to the gradient's x-coordinate
//! (`t = random`), and the gradient's y-midpoint is used. Gradient image
//! dimensions don't need to match the flood-fill image.

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

/// Map a flood-fill data image through a gradient, producing per-cell colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternFloodFillMapper {}

impl OpImagePatternFloodFillMapper {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flood fill mapper".to_string(),
            description: "Colours each flood-fill cell by sampling a gradient at the cell's random value; outside pixels pass through black.".to_string(),
            help: "Reads the packed data image from the flood fill node and looks up each cell's colour by horizontally sampling the gradient image at a per-cell t coordinate (bilinear along the gradient's y-midpoint).\n\nRandomness blends between using the sequential cell index (0) and the cell's stable random value (1) as t, while offset shifts every sample along the gradient. t is clamped to [0, 1] so endpoints sample the first/last gradient pixel rather than wrapping. Gradient dimensions do not need to match the input; pixels with cell id 0 are written as transparent black.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("flood fill".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Flood-fill data image produced by the flood fill node."),
            Input::new("gradient".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Horizontal gradient sampled per cell to pick its color."),
            Input::new("randomness".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Blends between using cell index (0) and per-cell random value (1) to sample the gradient."),
            Input::new("offset".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Shifts the gradient sample position for every cell."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA image with each cell filled by its sampled gradient color."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let ff_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let gradient_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let randomness_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let offset_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: ff, change_id: _ } = ff_converted.unwrap() else { unreachable!() };
        let Value::Image { data: gradient, change_id: _ } = gradient_converted.unwrap() else { unreachable!() };
        let Value::Decimal(randomness) = randomness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset) = offset_converted.unwrap() else { unreachable!() };

        let (width, height) = ff.dimensions();
        let g_ch = gradient.channels() as usize;
        let g_w = gradient.width();
        let g_h = gradient.height();
        let g_y = if g_h == 0 { 0.0 } else { (g_h as f32 - 1.0) * 0.5 };

        let mut output = FloatImage::new(width, height, 4);
        let mut sample_buf = [0.0f32; 4];
        for y in 0..height {
            for x in 0..width {
                let ff_px = ff.get_pixel(x, y);
                let id = ff_px[0];
                if id <= 0.0 {
                    output.put_pixel(x, y, &[0.0, 0.0, 0.0, 0.0]);
                    continue;
                }
                // t in [0,1]: blend between constant id-based (randomness=0)
                // and fully random (randomness=1), then offset. Clamp rather
                // than wrap so t=1 samples the last gradient pixel instead
                // of jumping back to the first.
                let random = ff_px[1];
                let t = (id * (1.0 - randomness) + random * randomness + offset).clamp(0.0, 1.0);
                let gx = t * g_w.saturating_sub(1).max(1) as f32;
                gradient.bilinear_sample(gx, g_y, &mut sample_buf[..g_ch]);
                let px = match g_ch {
                    1 => [sample_buf[0], sample_buf[0], sample_buf[0], 1.0],
                    2 => [sample_buf[0], sample_buf[0], sample_buf[0], sample_buf[1]],
                    3 => [sample_buf[0], sample_buf[1], sample_buf[2], 1.0],
                    _ => [sample_buf[0], sample_buf[1], sample_buf[2], sample_buf[3]],
                };
                output.put_pixel(x, y, &px);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "flood_fill_mapper_tests.rs"]
mod tests;
