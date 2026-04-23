//! Kuwahara filter operation for images.
//!
//! The Kuwahara filter is an edge-preserving smoothing filter that produces a
//! painterly, posterised look while keeping sharp transitions between regions
//! intact. For each pixel, it examines four overlapping square sub-regions
//! (top-left, top-right, bottom-left, bottom-right) of size `(radius+1) x (radius+1)`
//! around the pixel, computes the mean color and the luminance variance of each,
//! and outputs the mean of whichever sub-region has the lowest variance.
//!
//! The alpha channel (if present) is averaged alongside color channels, but the
//! variance used for region selection is based purely on luminance.

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

/// Kuwahara edge-preserving smoothing filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentKuwahara {}

impl OpImageAdjustmentKuwahara {
    /// Returns the node metadata (name and description) for the Kuwahara operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "kuwahara".to_string(),
            description: "Edge-preserving smoothing filter that produces a painterly look.".to_string(),
        }
    }

    /// Creates the input ports: image and radius controlling the size of each quadrant.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // radius is the half-size of each quadrant; quadrants are (radius+1) x (radius+1) pixels
            Input::new("radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the Kuwahara-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the Kuwahara filter. Edge pixels are clamped to the image bounds.
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

        // clamp radius to at least 1 — a radius of 0 would make each quadrant a single pixel and the filter would be a no-op
        let radius = radius.max(1) as i32;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        // for RGBA (4) or gray+alpha (2), treat the last channel as alpha and exclude it from variance computation
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };
        let data_ref = &data;
        let w = width as i32;
        let h = height as i32;

        // Process rows in parallel; each row returns its flat pixel buffer
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let mut row_pixels = Vec::with_capacity(w as usize * ch);

            for x in 0..w {
                // Evaluate all four overlapping quadrants around (x, y).
                // Each quadrant spans (radius+1) pixels in each axis and includes
                // the center pixel itself at a shared corner.
                // Quadrant offsets: (-r..=0, -r..=0), (0..=r, -r..=0), (-r..=0, 0..=r), (0..=r, 0..=r)
                let quadrants: [(i32, i32, i32, i32); 4] = [
                    (x - radius, y - radius, x,           y          ), // top-left
                    (x,          y - radius, x + radius,  y          ), // top-right
                    (x - radius, y,          x,           y + radius ), // bottom-left
                    (x,          y,          x + radius,  y + radius ), // bottom-right
                ];

                let mut best_variance = f32::INFINITY;
                let mut best_mean = vec![0.0f32; ch];

                for (x0, y0, x1, y1) in quadrants.iter() {
                    // clamp the quadrant bounds into the image (inclusive)
                    let cx0 = (*x0).clamp(0, w - 1);
                    let cy0 = (*y0).clamp(0, h - 1);
                    let cx1 = (*x1).clamp(0, w - 1);
                    let cy1 = (*y1).clamp(0, h - 1);

                    let mut sum = vec![0.0f64; ch];
                    let mut lum_sum: f64 = 0.0;
                    let mut lum_sum_sq: f64 = 0.0;
                    let mut count: u32 = 0;

                    for py in cy0..=cy1 {
                        for px in cx0..=cx1 {
                            let pixel = data_ref.get_pixel(px as u32, py as u32);
                            // accumulate per-channel sums (including alpha if present)
                            for c in 0..ch {
                                sum[c] += pixel[c] as f64;
                            }
                            // luminance used for variance: Rec. 709 for RGB, or the single channel for grayscale
                            let lum = if color_ch >= 3 {
                                0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
                            } else {
                                pixel[0]
                            } as f64;
                            lum_sum += lum;
                            lum_sum_sq += lum * lum;
                            count += 1;
                        }
                    }

                    // compute mean luminance and population variance E[X^2] - E[X]^2
                    let inv_n = 1.0 / count as f64;
                    let mean_lum = lum_sum * inv_n;
                    let variance = (lum_sum_sq * inv_n - mean_lum * mean_lum).max(0.0) as f32;

                    // keep the quadrant with the smallest luminance variance
                    if variance < best_variance {
                        best_variance = variance;
                        for c in 0..ch {
                            best_mean[c] = (sum[c] * inv_n) as f32;
                        }
                    }
                }

                // write the winning quadrant's mean to the output
                row_pixels.extend_from_slice(&best_mean);
            }
            row_pixels
        }).collect();

        // assemble the output FloatImage from the flat pixel buffer
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
#[path = "kuwahara_tests.rs"]
mod tests;
