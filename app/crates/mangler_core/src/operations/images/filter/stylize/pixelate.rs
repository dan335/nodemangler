//! Pixelate (mosaic) — block-average downsample at output resolution.
//!
//! Divides the image into a regular grid of `cell_size × cell_size` blocks,
//! computes the average colour of each block, and fills every pixel in the
//! block with that average. Produces the classic chunky "mosaic" look
//! without any actual resolution change — the output image has the same
//! dimensions as the input.

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

/// Mosaic / pixelate: block-average each cell to a single colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentPixelate {}

impl OpImageAdjustmentPixelate {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "pixelate".to_string(),
            description: "Block-averages the image on a regular grid to produce a mosaic.".to_string(),
            help: "Partitions the image into `cell_size` × `cell_size` blocks (the bottom/right edge block is clipped if it doesn't divide evenly), averages every channel inside each block, and fills the entire block with that average. Output dimensions and channel count match the input exactly — nothing is rescaled, just quantised into coarse blocks.\n\n`cell_size = 1` is a no-op. The average is plain arithmetic mean, so alpha channels average alongside colours; if you want an alpha-weighted colour average, pre-multiply before this node.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to pixelate."),
            Input::new("cell size".to_string(), Value::Integer(16), Some(InputSettings::Slider { range: (1.0, 256.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Side length of each mosaic block in pixels."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Pixelated image with each cell filled by its block-average colour."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let cell_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(cell) = cell_converted.unwrap() else { unreachable!() };

        let cell = cell.max(1) as u32;
        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let mut output = FloatImage::new(w, h, ch as u32);

        // One block at a time: compute mean, then flood-fill that block.
        let mut sum = [0.0f32; 4];
        let mut y0 = 0;
        while y0 < h {
            let y1 = (y0 + cell).min(h);
            let mut x0 = 0;
            while x0 < w {
                let x1 = (x0 + cell).min(w);
                // Accumulate across the block.
                for val in sum.iter_mut().take(ch) { *val = 0.0; }
                let count = ((x1 - x0) * (y1 - y0)) as f32;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let p = data.get_pixel(xx, yy);
                        for c in 0..ch { sum[c] += p[c]; }
                    }
                }
                for val in sum.iter_mut().take(ch) { *val /= count; }
                // Write the block mean into every pixel of the block.
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        output.put_pixel(xx, yy, &sum[..ch]);
                    }
                }
                x0 = x1;
            }
            y0 = y1;
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
#[path = "pixelate_tests.rs"]
mod tests;
