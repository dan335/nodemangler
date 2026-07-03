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
            help: "Classic Kuwahara (1976). For each pixel, evaluates four overlapping square quadrants (TL, TR, BL, BR) of size (radius+1), computes per-channel mean and luminance variance in each, and outputs the mean of whichever quadrant has the lowest luminance variance.\n\nProduces a flat, posterized, oil-painting aesthetic because flat-region quadrants win and their means replace neighboring gradients. Alpha is averaged with color but excluded from the variance metric. Rows are processed in parallel; edges clamped.".to_string(),
        }
    }

    /// Creates the input ports: image and radius controlling the size of each quadrant.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smooth with classic Kuwahara quadrant averaging."),
            // radius is the half-size of each quadrant; quadrants are (radius+1) x (radius+1) pixels
            Input::new("radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of each quadrant in pixels; larger values yield a chunkier painterly look."),
        ]
    }

    /// Creates the output port: the Kuwahara-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Painterly Kuwahara-smoothed image with preserved edges."),
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
        let radius = radius.max(1);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        // for RGBA (4) or gray+alpha (2), treat the last channel as alpha and exclude it from variance computation
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };
        let data_ref = &data;
        let w = width as i32;
        let h = height as i32;

        // Precompute summed-area tables so each quadrant's mean and variance
        // is an O(1) box lookup instead of an O((r+1)²) scan. Planes per cell:
        // one per channel, plus luminance and luminance².
        let n_planes = ch + 2;
        let lum_plane = ch;
        let lum2_plane = ch + 1;
        let stride = (w as usize + 1) * n_planes;
        let mut sat = vec![0.0f64; (h as usize + 1) * stride]; // row 0 / col 0 stay zero

        // Row pass (parallel): per-row prefix sums of every plane.
        sat[stride..].par_chunks_mut(stride).enumerate().for_each(|(y, out_row)| {
            let mut run = [0.0f64; 6]; // n_planes <= 6
            for x in 0..w as usize {
                let pixel = data_ref.get_pixel(x as u32, y as u32);
                for c in 0..ch {
                    run[c] += pixel[c] as f64;
                }
                // luminance used for variance: Rec. 709 for RGB, or the single channel for grayscale
                let lum = if color_ch >= 3 {
                    0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
                } else {
                    pixel[0]
                } as f64;
                run[lum_plane] += lum;
                run[lum2_plane] += lum * lum;
                let base = (x + 1) * n_planes;
                out_row[base..base + n_planes].copy_from_slice(&run[..n_planes]);
            }
        });
        // Column pass: accumulate rows top-to-bottom (cheap vectorized adds).
        for y in 1..=h as usize {
            let (prev, cur) = sat.split_at_mut(y * stride);
            let prev_row = &prev[(y - 1) * stride..];
            for (c_val, p_val) in cur[..stride].iter_mut().zip(prev_row.iter()) {
                *c_val += *p_val;
            }
        }
        let sat_ref = &sat;

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
                let mut best_mean = [0.0f32; 4];

                for (x0, y0, x1, y1) in quadrants.iter() {
                    // clamp the quadrant bounds into the image (inclusive)
                    let cx0 = (*x0).clamp(0, w - 1) as usize;
                    let cy0 = (*y0).clamp(0, h - 1) as usize;
                    let cx1 = (*x1).clamp(0, w - 1) as usize;
                    let cy1 = (*y1).clamp(0, h - 1) as usize;

                    let count = ((cx1 - cx0 + 1) * (cy1 - cy0 + 1)) as u32;

                    // O(1) inclusive box sums for every plane from the SATs
                    let br = (cy1 + 1) * stride + (cx1 + 1) * n_planes;
                    let tr = cy0 * stride + (cx1 + 1) * n_planes;
                    let bl = (cy1 + 1) * stride + cx0 * n_planes;
                    let tl = cy0 * stride + cx0 * n_planes;
                    let mut sums = [0.0f64; 6];
                    for (pl, sum) in sums.iter_mut().enumerate().take(n_planes) {
                        *sum = sat_ref[br + pl] - sat_ref[tr + pl] - sat_ref[bl + pl] + sat_ref[tl + pl];
                    }

                    // compute mean luminance and population variance E[X^2] - E[X]^2
                    let inv_n = 1.0 / count as f64;
                    let mean_lum = sums[lum_plane] * inv_n;
                    let variance = (sums[lum2_plane] * inv_n - mean_lum * mean_lum).max(0.0) as f32;

                    // keep the quadrant with the smallest luminance variance
                    if variance < best_variance {
                        best_variance = variance;
                        for c in 0..ch {
                            best_mean[c] = (sums[c] * inv_n) as f32;
                        }
                    }
                }

                // write the winning quadrant's mean to the output
                row_pixels.extend_from_slice(&best_mean[..ch]);
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
