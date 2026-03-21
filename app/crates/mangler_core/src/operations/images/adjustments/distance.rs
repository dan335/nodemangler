//! Distance field computation for images.
//!
//! Converts a grayscale image into a signed distance field by thresholding
//! pixels into inside/outside regions, then computing the minimum Euclidean
//! distance to the nearest boundary pixel. Output is normalized with 0.5 at
//! the boundary, values above 0.5 for inside regions, and below 0.5 for outside.

use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use rayon::prelude::*;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Distance field operation that computes a signed distance from a binary threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDistance{}

impl OpImageAdjustmentDistance {
    /// Returns the node metadata (name and description) for the distance operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "distance".to_string(),
            description: "Computes distance field from a binary image.".to_string(),
        }
    }

    /// Creates the input ports: image, luminance threshold for the binary mask, and spread
    /// (maximum search radius in pixels).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("spread".to_string(), Value::Decimal(32.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 256.0)) }), None),
        ]
    }

    /// Creates the output port: the distance field image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the distance field computation using brute-force nearest-boundary search
    /// within the spread radius.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let spread_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spread) = spread_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let threshold = threshold;
        let spread = spread.max(1.0);
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;
        let spread_i = spread.ceil() as i32;

        // threshold the image: compute binary mask
        let inside: Vec<bool> = buffer.pixels().map(|pixel| {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            lum >= threshold
        }).collect();

        // Compute distance transform and output image in parallel.
        let inside_ref = &inside;
        let w = width as usize;
        let h = height as usize;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).flat_map(move |x| {
                let idx = y * w + x;
                let is_inside = inside_ref[idx];
                let mut min_dist_sq = spread * spread;

                let y_start = (y as i32 - spread_i).max(0) as usize;
                let y_end = ((y as i32 + spread_i).min(height - 1)) as usize;
                let x_start = (x as i32 - spread_i).max(0) as usize;
                let x_end = ((x as i32 + spread_i).min(width - 1)) as usize;

                'outer: for sy in y_start..=y_end {
                    for sx in x_start..=x_end {
                        let sidx = sy * w + sx;
                        if inside_ref[sidx] != is_inside {
                            let ddx = (sx as f32) - (x as f32);
                            let ddy = (sy as f32) - (y as f32);
                            let dist_sq = ddx * ddx + ddy * ddy;
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                                if dist_sq <= 1.0 { break 'outer; }
                            }
                        }
                    }
                }

                let dist = min_dist_sq.sqrt();
                let normalized_dist = (dist / spread).clamp(0.0, 1.0);
                let result = if is_inside {
                    0.5 + normalized_dist / 2.0
                } else {
                    0.5 - normalized_dist / 2.0
                }.clamp(0.0, 1.0);

                [result, result, result, 1.0]
            })
        }).collect();

        let out_buffer = image::Rgba32FImage::from_raw(width as u32, height as u32, pixels).unwrap();
        let adjusted = DynamicImage::ImageRgba32F(out_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "distance_tests.rs"]
mod tests;
