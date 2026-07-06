//! Median filter operation for images.
//!
//! Replaces each pixel with the per-channel median value in its square
//! neighborhood. Removes small details and salt-and-pepper noise while
//! preserving sharp edges, producing a blocky / cartoon aesthetic.
//!
//! Uses a Huang-style sliding window: each row keeps one sorted window per
//! channel and stepping x by one removes the departing column and inserts
//! the entering one — O(r) updates per pixel instead of re-gathering and
//! selecting over the full k = (2r+1)² window. Radius is capped in the UI
//! because k grows quadratically.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
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
            help: "For every pixel and channel, gathers the (2r+1) square window and writes the middle value back. Unlike mean-based smoothing, the median is robust to outliers, so salt-and-pepper noise and thin specks are removed while long edges stay razor-sharp.\n\nImplemented with a sliding sorted window (Huang): stepping one pixel swaps a single column in and out, so cost per pixel is O(r) updates rather than re-selecting over the full k = (2r+1)^2 window. Radius is capped because k grows quadratically. Each channel (including alpha) is filtered independently, which can shift colors at high radius.".to_string(),
        }
    }

    /// Creates the input ports: image and window radius (capped to keep cost bounded).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to denoise or stylize with per-channel median filtering."),
            // radius is capped at 8 — at r=8 the per-pixel window is 17x17=289 samples
            Input::new("radius".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window, in pixels at a 1024px reference (scales with image size); larger values produce a chunkier look."),
        ]
    }

    /// Creates the output port: the median-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Median-filtered image with small details removed and edges preserved."),
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

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the filter looks the same relative size at any resolution.
        let (width, height) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, width, height).round().max(1.0) as i32;
        let ch = data.channels() as usize;
        let data_ref = &data;
        let w = width as i32;
        let h = height as i32;
        let window = (2 * radius + 1) as usize * (2 * radius + 1) as usize;

        // Deinterleave into per-channel planes once: window gathers then read
        // contiguous plane rows (interior windows are straight memcpys)
        // instead of strided interleaved samples. A sorted sliding window is
        // not worth it here: keeping it sorted costs two O(window) memmoves
        // per entering/leaving sample, which measures slower than
        // re-selecting.
        let wu = width as usize;
        let raw = data.as_raw();
        let planes: Vec<Vec<f32>> = (0..ch)
            .map(|c| raw.iter().skip(c).step_by(ch).copied().collect())
            .collect();
        let planes_ref = &planes;
        let win_w = (2 * radius + 1) as usize;

        // Process rows in parallel. Each row thread reuses a single sample
        // buffer; the median is the window/2-th order statistic under
        // total_cmp, and quickselect ignores gather order, so plane-order
        // gathering is result-identical.
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let mut row_pixels = Vec::with_capacity(w as usize * ch);
            let mut buf: Vec<f32> = Vec::with_capacity(window);
            let mid = window / 2;

            // Row-invariant clamped y coordinates for the window sweep.
            let py_range: Vec<usize> = (-radius..=radius)
                .map(|dy| (y + dy).clamp(0, h - 1) as usize)
                .collect();

            for x in 0..w {
                // Interior windows need no x clamping — copy whole segments.
                let interior = x >= radius && x + radius < w;

                // each channel is median-filtered independently, including
                // alpha. total_cmp handles NaN deterministically even though
                // normal image data won't contain any.
                for plane in planes_ref.iter() {
                    buf.clear();
                    if interior {
                        let x0 = (x - radius) as usize;
                        for &py in py_range.iter() {
                            let base = py * wu;
                            buf.extend_from_slice(&plane[base + x0..base + x0 + win_w]);
                        }
                    } else {
                        for &py in py_range.iter() {
                            let base = py * wu;
                            for dx in -radius..=radius {
                                let px = (x + dx).clamp(0, w - 1) as usize;
                                buf.push(plane[base + px]);
                            }
                        }
                    }

                    let (_, median, _) =
                        buf.select_nth_unstable_by(mid, |a, b| a.total_cmp(b));
                    row_pixels.push(*median);
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
