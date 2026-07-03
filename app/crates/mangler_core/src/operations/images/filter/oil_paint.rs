//! Oil paint stylization filter.
//!
//! Classic intensity-histogram oil-paint effect (Holzmann 1988):
//!   1. For each pixel, look at a square neighborhood.
//!   2. Quantize each neighbor's luminance into one of `levels` bins.
//!   3. Find the most populated bin.
//!   4. Output the average color of the neighbors that fell into that bin.
//!
//! The result looks like brush strokes painted with a limited palette:
//! smooth, posterized patches separated by hard boundaries where the dominant
//! bin changes. Different in feel from Kuwahara (variance-based) and from
//! toon (luminance-posterize + edge overlay).

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

/// Oil paint stylization filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentOilPaint {}

impl OpImageAdjustmentOilPaint {
    /// Returns the node metadata for the oil paint filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "oil paint".to_string(),
            description: "Oil-paint stylization via intensity-histogram quantization of each neighborhood.".to_string(),
            help: "Holzmann 1988 intensity-histogram oil paint. For every pixel, gathers a square brush neighborhood, quantizes each neighbor's luminance into one of `levels` bins, finds the dominant bin, and outputs the average color of the neighbors that fell into it.\n\nSmooth posterized patches separated by hard boundaries where the dominant bin flips produce the brush-stroke look. Fewer levels means a more posterized palette; larger radius gives chunkier strokes. Different character from Kuwahara (variance-based) or toon (posterize + edge). Alpha is copied from the center pixel; rows run in parallel.".to_string(),
        }
    }

    /// Creates input ports: image, brush radius, and number of intensity bins.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to stylize with brush-stroke oil-paint posterization."),
            // Brush radius — larger = chunkier strokes
            Input::new("radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 10.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Brush radius in pixels; larger values produce chunkier brush strokes."),
            // Number of intensity bins — lower = more posterized palette
            Input::new("levels".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (2.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of luminance bins; fewer levels give a more posterized palette."),
        ]
    }

    /// Creates the output port: the stylized image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Oil-paint stylized image built from dominant-bin neighborhood averages."),
        ]
    }

    /// Runs the oil paint filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let levels_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        let radius = radius.max(1);
        let levels = levels.max(2) as usize;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let w = width as i32;
        let h = height as i32;
        // Alpha (if present) is copied through from the center pixel rather than binned
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        let data_ref = &data;
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let mut row = Vec::with_capacity(w as usize * ch);

            // Sliding histogram over the window: stepping x by one only swaps
            // one (clamped) column in and one out — O(r) per pixel instead of
            // re-binning the whole (2r+1)² window. Sums are f64 so repeated
            // add/subtract cannot drift.
            let mut counts = vec![0u32; levels];
            let mut sums = vec![[0.0f64; 4]; levels];

            // Map luminance to bin index in [0, levels-1]
            let bin_of = |p: &[f32]| -> usize {
                let lum = if color_ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                };
                ((lum.clamp(0.0, 1.0) * (levels as f32 - 1.0)).round() as usize).min(levels - 1)
            };

            for x in 0..w {
                if x == 0 {
                    // Build the initial histogram over all window columns
                    for dx in -radius..=radius {
                        let px = dx.clamp(0, w - 1) as u32;
                        for dy in -radius..=radius {
                            let py = (y + dy).clamp(0, h - 1) as u32;
                            let p = data_ref.get_pixel(px, py);
                            let bin = bin_of(p);
                            counts[bin] += 1;
                            for c in 0..color_ch {
                                sums[bin][c] += p[c] as f64;
                            }
                        }
                    }
                } else {
                    // Slide: subtract the departing column, add the entering one.
                    // Clamped columns shift consistently, so the window multiset
                    // changes by exactly these two columns.
                    let out_col = (x - 1 - radius).clamp(0, w - 1) as u32;
                    let in_col = (x + radius).clamp(0, w - 1) as u32;
                    for dy in -radius..=radius {
                        let py = (y + dy).clamp(0, h - 1) as u32;
                        let p_out = data_ref.get_pixel(out_col, py);
                        let b_out = bin_of(p_out);
                        counts[b_out] -= 1;
                        for c in 0..color_ch {
                            sums[b_out][c] -= p_out[c] as f64;
                        }
                        let p_in = data_ref.get_pixel(in_col, py);
                        let b_in = bin_of(p_in);
                        counts[b_in] += 1;
                        for c in 0..color_ch {
                            sums[b_in][c] += p_in[c] as f64;
                        }
                    }
                }

                // Find the most populated bin (ties broken toward the first bin)
                let mut best = 0usize;
                for b in 1..levels {
                    if counts[b] > counts[best] { best = b; }
                }

                // Emit averaged color from the winning bin; alpha copied from center
                let center = data_ref.get_pixel(x.clamp(0, w - 1) as u32, y.clamp(0, h - 1) as u32);
                let n = counts[best].max(1) as f64;
                for val in sums[best].iter().take(color_ch) {
                    row.push((val / n) as f32);
                }
                if ch == 2 || ch == 4 {
                    row.push(center[ch - 1]);
                }
            }
            row
        }).collect();

        let out = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "oil_paint_tests.rs"]
mod tests;
