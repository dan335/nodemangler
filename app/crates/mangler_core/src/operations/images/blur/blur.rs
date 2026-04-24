//! Gaussian blur operation for images.
//!
//! Approximates a Gaussian blur using three successive box blur passes
//! (horizontal + vertical each). This is O(n) per pixel regardless of
//! sigma and closely matches a true Gaussian distribution.
//!
//! Works directly on [`FloatImage`] f32 data, avoiding any u8 conversions.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Gaussian blur operation that smooths an image using a 3-pass box blur approximation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentBlur {}

impl OpImageAdjustmentBlur {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blur".to_string(),
            description: "Applies a Gaussian blur with adjustable radius.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the Gaussian blur. The input FloatImage data is already f32,
    /// so we work directly on a flat buffer without any u8 conversion.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sigma) = sigma_converted.unwrap() else { unreachable!() };

        let sigma = sigma.max(0.0);

        // Zero sigma means no blur — return the original image unchanged
        if sigma < f32::EPSILON {
            return Ok(OperationResponse { 
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                ],
            });
        }

        let output = gaussian_blur_image(&data, sigma);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Apply a Gaussian blur to a [`FloatImage`] using a 3-pass box blur approximation.
///
/// Shared helper exposed so other operations (highpass, glows, bevel smoothing)
/// can reuse the same blur implementation without going through the full input
/// plumbing of [`OpImageAdjustmentBlur`]. For `sigma <= 0`, returns a clone of
/// the input.
pub(crate) fn gaussian_blur_image(data: &FloatImage, sigma: f32) -> FloatImage {
    let sigma = sigma.max(0.0);
    if sigma < f32::EPSILON {
        return data.clone();
    }

    let ch = data.channels() as usize;
    let (width, height) = data.dimensions();
    let len = (width * height) as usize;

    let boxes = box_sizes_for_gaussian(sigma, 3);

    let mut buf: Vec<f32> = data.as_raw().to_vec();
    let mut tmp = vec![0.0f32; len * ch];

    for radius in &boxes {
        box_blur_h(&buf, &mut tmp, width, height, ch, *radius);
        box_blur_v(&tmp, &mut buf, width, height, ch, *radius);
    }

    FloatImage::from_raw(width, height, data.channels(), buf).unwrap()
}

/// Compute box radii for an n-pass box blur that approximates a Gaussian with the given sigma.
/// Based on the W3C SVG specification algorithm.
fn box_sizes_for_gaussian(sigma: f32, n: usize) -> Vec<u32> {
    let w_ideal = (12.0 * sigma * sigma / n as f32 + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i32;
    if wl % 2 == 0 { wl -= 1; }
    wl = wl.max(1);
    let wu = wl + 2;

    let m = ((12.0 * sigma * sigma
        - 3.0 * (wl * wl) as f32
        - 12.0 * wl as f32
        - 9.0)
        / (-4.0 * wl as f32 - 4.0))
        .round().max(0.0) as usize;

    (0..n).map(|i| {
        let w = if i < m { wl } else { wu };
        ((w - 1) / 2).max(0) as u32
    }).collect()
}

/// Horizontal box blur pass using a running sum. Clamps at edges.
/// Works with dynamic channel count via the `channels` parameter.
fn box_blur_h(src: &[f32], dst: &mut [f32], width: u32, height: u32, channels: usize, radius: u32) {
    let r = radius as i32;
    let diam = (2 * r + 1) as f32;
    let w = width as i32;

    for y in 0..height as usize {
        let row = y * width as usize;

        // Initialize running sum by accumulating the first (2r+1) pixels (clamped)
        let mut sum = vec![0.0f32; channels];
        for ix in -r..=r {
            let x = ix.clamp(0, w - 1) as usize;
            let idx = (row + x) * channels;
            for c in 0..channels {
                sum[c] += src[idx + c];
            }
        }

        // Slide the window across the row
        for x in 0..w {
            let dst_idx = (row + x as usize) * channels;
            for c in 0..channels {
                dst[dst_idx + c] = sum[c] / diam;
            }

            // Add the new right pixel and remove the old left pixel
            let right = (x + r + 1).min(w - 1) as usize;
            let left = (x - r).max(0) as usize;
            let add_idx = (row + right) * channels;
            let rem_idx = (row + left) * channels;
            for c in 0..channels {
                sum[c] += src[add_idx + c] - src[rem_idx + c];
            }
        }
    }
}

/// Vertical box blur pass using a running sum. Clamps at edges.
/// Works with dynamic channel count via the `channels` parameter.
fn box_blur_v(src: &[f32], dst: &mut [f32], width: u32, height: u32, channels: usize, radius: u32) {
    let r = radius as i32;
    let diam = (2 * r + 1) as f32;
    let w = width as usize;
    let h = height as i32;

    for x in 0..width as usize {
        // Initialize running sum by accumulating the first (2r+1) pixels (clamped)
        let mut sum = vec![0.0f32; channels];
        for iy in -r..=r {
            let y = iy.clamp(0, h - 1) as usize;
            let idx = (y * w + x) * channels;
            for c in 0..channels {
                sum[c] += src[idx + c];
            }
        }

        // Slide the window down the column
        for y in 0..h {
            let dst_idx = (y as usize * w + x) * channels;
            for c in 0..channels {
                dst[dst_idx + c] = sum[c] / diam;
            }

            // Add the new bottom pixel and remove the old top pixel
            let bottom = (y + r + 1).min(h - 1) as usize;
            let top = (y - r).max(0) as usize;
            let add_idx = (bottom * w + x) * channels;
            let rem_idx = (top * w + x) * channels;
            for c in 0..channels {
                sum[c] += src[add_idx + c] - src[rem_idx + c];
            }
        }
    }
}

#[cfg(test)]
#[path = "blur_tests.rs"]
mod tests;
