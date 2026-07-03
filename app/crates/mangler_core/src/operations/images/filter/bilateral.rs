//! Bilateral filter operation for images.
//!
//! Edge-preserving smoothing via a weighted average of a pixel's neighborhood,
//! where each neighbor's weight is the product of:
//! - a spatial Gaussian (falls off with pixel distance), and
//! - a range Gaussian (falls off with color difference from the center pixel).
//!
//! Produces a "denoised photograph" look — flat regions become smooth while
//! edges stay sharp, because pixels on the far side of an edge have low range
//! weight and contribute little to the average. Complementary to the painterly
//! sector-averaging look of the Kuwahara filter.

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

/// Bilateral edge-preserving smoothing filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentBilateral {}

impl OpImageAdjustmentBilateral {
    /// Returns the node metadata (name and description) for the bilateral operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bilateral".to_string(),
            description: "Edge-preserving smoothing using combined spatial and color-similarity weights.".to_string(),
            help: "Replaces each pixel with a weighted average of its neighbors where the weight is the product of a spatial Gaussian (falls off with pixel distance) and a range Gaussian (falls off with color difference to the center). Pixels on the far side of an edge have low range weight and barely contribute, so edges stay sharp while flat areas denoise.\n\nSmaller `range sigma` preserves edges more aggressively; `spatial sigma` typically tracks radius. Cost is O(r^2) per pixel; rows run in parallel via rayon. Alpha is excluded from the color-difference metric but still averaged.".to_string(),
        }
    }

    /// Creates the input ports: image, radius, spatial sigma, and range (color) sigma.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smooth while keeping edges crisp."),
            // radius of the square window in pixels (full window is (2r+1) x (2r+1))
            Input::new("radius".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the sampling window in pixels; larger values average over a wider area."),
            // spatial sigma: controls how fast spatial weight falls off with distance; tends to track radius
            Input::new("spatial sigma".to_string(), Value::Decimal(2.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Falloff of the spatial Gaussian; larger values smooth further across distance."),
            // range sigma: controls how tolerant the filter is of color differences (smaller = more edge-preserving)
            Input::new("range sigma".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Color-similarity tolerance; smaller values preserve edges more strongly."),
        ]
    }

    /// Creates the output port: the bilaterally filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Bilaterally smoothed image with edges preserved."),
        ]
    }

    /// Executes the bilateral filter. Each pixel is replaced with the weighted
    /// average of its neighborhood using spatial * range Gaussian weights.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let spatial_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let range_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spatial_sigma) = spatial_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range_sigma) = range_converted.unwrap() else { unreachable!() };

        // clamp controls to valid ranges to avoid division-by-zero and negative widths
        let radius = radius.max(1);
        let spatial_sigma = spatial_sigma.max(1e-4);
        let range_sigma = range_sigma.max(1e-4);

        // precompute weight divisors once — weights are exp(-d^2 / (2 * sigma^2))
        let spatial_denom = 2.0 * spatial_sigma * spatial_sigma;
        let range_denom = 2.0 * range_sigma * range_sigma;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        // for RGBA or gray+alpha, exclude alpha from the range-weight color difference
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };
        let data_ref = &data;
        let w = width as i32;
        let h = height as i32;

        // precompute the spatial-weight table for (dx, dy) offsets in the window
        // size = (2r+1)^2 — cheap memory, saves one exp per neighbor per pixel
        let win = (2 * radius + 1) as usize;
        let mut spatial_table = vec![0.0f32; win * win];
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = (dx * dx + dy * dy) as f32;
                let idx = ((dy + radius) as usize) * win + (dx + radius) as usize;
                spatial_table[idx] = (-d2 / spatial_denom).exp();
            }
        }

        // precompute the range-weight LUT over quantized squared color distance,
        // sampled with linear interpolation — replaces one exp() per neighbor.
        // The domain covers weights down to exp(-13.8) ≈ 1e-6; anything past
        // the end (color_d2 is unbounded for HDR-ish inputs) contributes a
        // negligible weight and is treated as zero.
        const RANGE_LUT_SIZE: usize = 2048;
        let lut_max_d2 = range_denom * 13.815511; // -ln(1e-6)
        let lut_scale = (RANGE_LUT_SIZE - 1) as f32 / lut_max_d2;
        let mut range_lut = [0.0f32; RANGE_LUT_SIZE];
        for (i, entry) in range_lut.iter_mut().enumerate() {
            let d2 = i as f32 / lut_scale;
            *entry = (-d2 / range_denom).exp();
        }

        // Process each row in parallel
        let spatial_table_ref = &spatial_table;
        let range_lut_ref = &range_lut;
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let mut row_pixels = Vec::with_capacity(w as usize * ch);

            for x in 0..w {
                let center = data_ref.get_pixel(x as u32, y as u32);

                let mut sum = [0.0f32; 4];
                let mut weight_sum: f32 = 0.0;

                // sweep over the square window, clamping to image bounds
                for dy in -radius..=radius {
                    let py = (y + dy).clamp(0, h - 1);
                    let row_idx = ((dy + radius) as usize) * win;
                    for dx in -radius..=radius {
                        let px = (x + dx).clamp(0, w - 1);
                        let neighbor = data_ref.get_pixel(px as u32, py as u32);

                        // range weight = gauss(|color(center) - color(neighbor)|)
                        // using color channels only (alpha excluded from similarity)
                        let mut color_d2 = 0.0f32;
                        for c in 0..color_ch {
                            let d = center[c] - neighbor[c];
                            color_d2 += d * d;
                        }
                        let spatial_w = spatial_table_ref[row_idx + (dx + radius) as usize];
                        // range weight via LUT with linear interpolation
                        let t = color_d2 * lut_scale;
                        let range_w = if t >= (RANGE_LUT_SIZE - 1) as f32 {
                            0.0
                        } else {
                            let i = t as usize;
                            let frac = t - i as f32;
                            range_lut_ref[i] + (range_lut_ref[i + 1] - range_lut_ref[i]) * frac
                        };
                        let weight = spatial_w * range_w;

                        for c in 0..ch {
                            sum[c] += neighbor[c] * weight;
                        }
                        weight_sum += weight;
                    }
                }

                // normalize — weight_sum is guaranteed > 0 because the center pixel contributes w=1
                let inv_w = 1.0 / weight_sum;
                for val in sum.iter().take(ch) {
                    row_pixels.push(val * inv_w);
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
#[path = "bilateral_tests.rs"]
mod tests;
