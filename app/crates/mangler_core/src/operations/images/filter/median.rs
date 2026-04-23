//! Median filter operation for images.
//!
//! Replaces each pixel with the per-channel median value in its square
//! neighborhood. Removes small details and salt-and-pepper noise while
//! preserving sharp edges, producing a blocky / cartoon aesthetic.
//!
//! Uses `select_nth_unstable_by` for a Quickselect-style median lookup so
//! the cost per pixel is O(k) average where k = (2r+1)² rather than O(k log k)
//! of a full sort. Radius is capped in the UI because k grows quadratically.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Median filter for edge-preserving smoothing with a cartoon/blocky aesthetic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentMedian {}

impl OpImageAdjustmentMedian {
    /// Returns the node metadata (name and description) for the median operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "median".to_string(),
            description: "Replaces each pixel with the per-channel median of its neighborhood. Preserves edges, removes small details.".to_string(),
        }
    }

    /// Creates the input ports: image and window radius (capped to keep cost bounded).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // radius is capped at 8 — at r=8 the per-pixel window is 17x17=289 samples
            Input::new("radius".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the median-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the median filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        let radius = radius.max(1) as i32;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let data_ref = &data;
        let w = width as i32;
        let h = height as i32;
        let window = (2 * radius + 1) as usize * (2 * radius + 1) as usize;

        // Process rows in parallel. Each row thread reuses a single sample
        // buffer across the row to avoid per-pixel allocations.
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let mut row_pixels = Vec::with_capacity(w as usize * ch);
            let mut buf: Vec<f32> = Vec::with_capacity(window);

            for x in 0..w {
                // each channel is median-filtered independently, including alpha
                for c in 0..ch {
                    buf.clear();
                    for dy in -radius..=radius {
                        let py = (y + dy).clamp(0, h - 1) as u32;
                        for dx in -radius..=radius {
                            let px = (x + dx).clamp(0, w - 1) as u32;
                            buf.push(data_ref.get_pixel(px, py)[c]);
                        }
                    }
                    // Quickselect the middle element (population median).
                    // total_cmp handles NaN deterministically even though normal
                    // image data won't contain any.
                    let mid = buf.len() / 2;
                    let (_, pivot, _) = buf.select_nth_unstable_by(mid, |a, b| a.total_cmp(b));
                    row_pixels.push(*pivot);
                }
            }
            row_pixels
        }).collect();

        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "median_tests.rs"]
mod tests;
