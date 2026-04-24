//! Canny edge detector.
//!
//! Implements the classic multi-stage edge detection algorithm:
//!   1. Gaussian smoothing of the luminance channel.
//!   2. Sobel gradient magnitude and direction.
//!   3. Non-maximum suppression along the gradient direction (quantized to
//!      4 angles: 0°, 45°, 90°, 135°).
//!   4. Double thresholding with hysteresis: strong edges are kept, weak
//!      edges are kept only if they are 8-connected to a strong edge.
//!
//! The output is a binary edge mask (0 or 1) replicated across color
//! channels, with the input alpha preserved.

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

/// Canny multi-stage edge detector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentCanny {}

impl OpImageAdjustmentCanny {
    /// Returns the node metadata (name and description) for Canny.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "canny".to_string(),
            description: "Canny edge detector (Gaussian → Sobel → non-max suppression → hysteresis).".to_string(),
        }
    }

    /// Creates input ports: image, Gaussian sigma (smoothing), low and high
    /// thresholds for hysteresis.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // sigma for the pre-smoothing Gaussian
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 5.0), step_by: Some(0.1), clamp_to_range: true }), None),
            // lower threshold: below this, gradients are rejected
            Input::new("low threshold".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            // upper threshold: above this, gradients are always kept
            Input::new("high threshold".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: binary edge image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Runs Canny edge detection.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let low_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let high_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sigma) = sigma_converted.unwrap() else { unreachable!() };
        let Value::Decimal(low) = low_converted.unwrap() else { unreachable!() };
        let Value::Decimal(high) = high_converted.unwrap() else { unreachable!() };

        // Ensure high >= low so hysteresis has a sensible band
        let low = low.clamp(0.0, 1.0);
        let high = high.clamp(0.0, 1.0).max(low);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let w = width as i32;
        let h = height as i32;
        let n = (width * height) as usize;

        // --- stage 1: extract luminance & Gaussian-smooth ---
        let mut lum = vec![0.0f32; n];
        for y in 0..height {
            for x in 0..width {
                let p = data.get_pixel(x, y);
                lum[(y * width + x) as usize] = if ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                };
            }
        }
        let smoothed = gaussian_blur_planar(&lum, width, height, sigma.max(0.1));

        // --- stage 2: Sobel gradients ---
        let mut mag = vec![0.0f32; n];
        // Quantized direction code: 0 = east/west, 1 = NE/SW, 2 = north/south, 3 = NW/SE
        let mut dir = vec![0u8; n];
        let mut max_mag: f32 = 0.0;
        for y in 0..h {
            for x in 0..w {
                let x0 = (x - 1).max(0) as usize;
                let x1 = x as usize;
                let x2 = (x + 1).min(w - 1) as usize;
                let y0 = (y - 1).max(0) as usize;
                let y1 = y as usize;
                let y2 = (y + 1).min(h - 1) as usize;
                let wu = width as usize;
                let s = |ix: usize, iy: usize| smoothed[iy * wu + ix];

                // Sobel Gx and Gy
                let gx = -s(x0, y0) - 2.0 * s(x0, y1) - s(x0, y2)
                        + s(x2, y0) + 2.0 * s(x2, y1) + s(x2, y2);
                let gy = -s(x0, y0) - 2.0 * s(x1, y0) - s(x2, y0)
                        + s(x0, y2) + 2.0 * s(x1, y2) + s(x2, y2);

                let m = (gx * gx + gy * gy).sqrt();
                let idx = y1 * wu + x1;
                mag[idx] = m;
                if m > max_mag { max_mag = m; }

                // Quantize gradient angle into one of 4 bins
                dir[idx] = quantize_angle(gx, gy);
            }
        }

        // Normalize magnitude to [0, 1] using the observed max so thresholds
        // are meaningful across different input dynamic ranges. We require a
        // small absolute minimum before normalizing — otherwise floating-point
        // noise in Gaussian+Sobel on a nearly-flat image would be amplified
        // into false edges.
        if max_mag > 1e-4 {
            for m in &mut mag { *m /= max_mag; }
        } else {
            for m in &mut mag { *m = 0.0; }
        }

        // --- stage 3: non-maximum suppression ---
        let mut nms = vec![0.0f32; n];
        let wu = width as usize;
        for y in 1..(h - 1) {
            for x in 1..(w - 1) {
                let idx = y as usize * wu + x as usize;
                let m = mag[idx];
                let (n1, n2) = match dir[idx] {
                    // 0 = east/west → compare with left & right neighbors
                    0 => (mag[idx - 1], mag[idx + 1]),
                    // 1 = NE/SW → compare with (x+1, y-1) and (x-1, y+1)
                    1 => (mag[idx - wu + 1], mag[idx + wu - 1]),
                    // 2 = north/south → compare with top & bottom
                    2 => (mag[idx - wu], mag[idx + wu]),
                    // 3 = NW/SE → compare with (x-1, y-1) and (x+1, y+1)
                    _ => (mag[idx - wu - 1], mag[idx + wu + 1]),
                };
                if m >= n1 && m >= n2 {
                    nms[idx] = m;
                }
            }
        }

        // --- stage 4: hysteresis thresholding ---
        // 0 = reject, 1 = weak (may be promoted), 2 = strong
        let mut mark = vec![0u8; n];
        let mut stack: Vec<usize> = Vec::new();
        for idx in 0..n {
            if nms[idx] >= high {
                mark[idx] = 2;
                stack.push(idx);
            } else if nms[idx] >= low {
                mark[idx] = 1;
            }
        }
        // Flood-fill strong → adjacent weak connections
        while let Some(idx) = stack.pop() {
            let x = (idx % wu) as i32;
            let y = (idx / wu) as i32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
                    let nidx = ny as usize * wu + nx as usize;
                    if mark[nidx] == 1 {
                        mark[nidx] = 2;
                        stack.push(nidx);
                    }
                }
            }
        }

        // --- build output image: strong marks → white, else black ---
        let mut out = FloatImage::new(width, height, ch as u32);
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let v = if mark[idx] == 2 { 1.0 } else { 0.0 };
                let src = data.get_pixel(x, y);
                let mut pixel = [0.0f32; 4];
                for c in 0..ch.min(3) { pixel[c] = v; }
                if ch == 2 || ch == 4 { pixel[ch - 1] = src[ch - 1]; }
                out.put_pixel(x, y, &pixel[..ch]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

/// Quantizes a gradient vector (gx, gy) into one of four bins:
/// 0 = east/west, 1 = NE/SW, 2 = north/south, 3 = NW/SE.
fn quantize_angle(gx: f32, gy: f32) -> u8 {
    // atan2 gives (-π, π]; we fold to [0, π) since opposite directions share a bin
    let mut a = gy.atan2(gx);
    if a < 0.0 { a += std::f32::consts::PI; }
    let deg = a.to_degrees();
    if deg < 22.5 || deg >= 157.5 { 0 }
    else if deg < 67.5 { 1 }
    else if deg < 112.5 { 2 }
    else { 3 }
}

/// Separable Gaussian blur on a single-channel planar buffer (truncated at 3σ).
fn gaussian_blur_planar(src: &[f32], width: u32, height: u32, sigma: f32) -> Vec<f32> {
    let sigma = sigma.max(1e-6);
    let radius = (3.0 * sigma).ceil() as i32;
    let mut kernel = vec![0.0f32; (2 * radius + 1) as usize];
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut sum = 0.0f32;
    for i in -radius..=radius {
        let w = (-((i * i) as f32) / two_sigma_sq).exp();
        kernel[(i + radius) as usize] = w;
        sum += w;
    }
    for w in &mut kernel { *w /= sum; }

    let w = width as i32;
    let h = height as i32;
    let mut tmp = vec![0.0f32; src.len()];
    for y in 0..h {
        let row = (y * w) as usize;
        for x in 0..w {
            let mut acc = 0.0f32;
            for k in -radius..=radius {
                let sx = (x + k).clamp(0, w - 1) as usize;
                acc += src[row + sx] * kernel[(k + radius) as usize];
            }
            tmp[row + x as usize] = acc;
        }
    }
    let mut out = vec![0.0f32; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f32;
            for k in -radius..=radius {
                let sy = (y + k).clamp(0, h - 1) as usize;
                acc += tmp[sy * w as usize + x as usize] * kernel[(k + radius) as usize];
            }
            out[(y * w + x) as usize] = acc;
        }
    }
    out
}

#[cfg(test)]
#[path = "canny_tests.rs"]
mod tests;
