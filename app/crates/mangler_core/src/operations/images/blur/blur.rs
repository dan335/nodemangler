//! Gaussian blur operation for images.
//!
//! Approximates a Gaussian blur using three successive box blur passes
//! (horizontal + vertical each). This is O(n) per pixel regardless of
//! sigma and closely matches a true Gaussian distribution.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use image::DynamicImage;
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
            description: "Blurs an image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sigma) = sigma_converted.unwrap() else { unreachable!() };

        let sigma = sigma.max(0.0);

        if sigma < f32::EPSILON {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::DynamicImage { data, change_id: get_id() } },
                ],
            });
        }

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();
        let len = (width * height) as usize;

        let boxes = box_sizes_for_gaussian(sigma, 3);

        // Work in f32 for precision across passes
        let mut buf: Vec<[f32; 4]> = rgba.pixels()
            .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32])
            .collect();
        let mut tmp = vec![[0.0f32; 4]; len];

        for radius in &boxes {
            box_blur_h(&buf, &mut tmp, width, height, *radius);
            box_blur_v(&tmp, &mut buf, width, height, *radius);
        }

        let mut output_buf = image::RgbaImage::new(width, height);
        for (i, pixel) in output_buf.pixels_mut().enumerate() {
            *pixel = image::Rgba([
                buf[i][0].round().clamp(0.0, 255.0) as u8,
                buf[i][1].round().clamp(0.0, 255.0) as u8,
                buf[i][2].round().clamp(0.0, 255.0) as u8,
                buf[i][3].round().clamp(0.0, 255.0) as u8,
            ]);
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(output_buf)), change_id: get_id() } },
            ],
        })
    }
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
fn box_blur_h(src: &[[f32; 4]], dst: &mut [[f32; 4]], width: u32, height: u32, radius: u32) {
    let r = radius as i32;
    let diam = (2 * r + 1) as f32;
    let w = width as i32;

    for y in 0..height as usize {
        let row = y * width as usize;

        let mut sum = [0.0f32; 4];
        for ix in -r..=r {
            let x = ix.clamp(0, w - 1) as usize;
            let px = src[row + x];
            sum[0] += px[0]; sum[1] += px[1]; sum[2] += px[2]; sum[3] += px[3];
        }

        for x in 0..w {
            dst[row + x as usize] = [sum[0] / diam, sum[1] / diam, sum[2] / diam, sum[3] / diam];

            let right = (x + r + 1).min(w - 1) as usize;
            let left = (x - r).max(0) as usize;
            let add = src[row + right];
            let rem = src[row + left];
            sum[0] += add[0] - rem[0];
            sum[1] += add[1] - rem[1];
            sum[2] += add[2] - rem[2];
            sum[3] += add[3] - rem[3];
        }
    }
}

/// Vertical box blur pass using a running sum. Clamps at edges.
fn box_blur_v(src: &[[f32; 4]], dst: &mut [[f32; 4]], width: u32, height: u32, radius: u32) {
    let r = radius as i32;
    let diam = (2 * r + 1) as f32;
    let w = width as usize;
    let h = height as i32;

    for x in 0..width as usize {
        let mut sum = [0.0f32; 4];
        for iy in -r..=r {
            let y = iy.clamp(0, h - 1) as usize;
            let px = src[y * w + x];
            sum[0] += px[0]; sum[1] += px[1]; sum[2] += px[2]; sum[3] += px[3];
        }

        for y in 0..h {
            dst[y as usize * w + x] = [sum[0] / diam, sum[1] / diam, sum[2] / diam, sum[3] / diam];

            let bottom = (y + r + 1).min(h - 1) as usize;
            let top = (y - r).max(0) as usize;
            let add = src[bottom * w + x];
            let rem = src[top * w + x];
            sum[0] += add[0] - rem[0];
            sum[1] += add[1] - rem[1];
            sum[2] += add[2] - rem[2];
            sum[3] += add[3] - rem[3];
        }
    }
}

#[cfg(test)]
#[path = "blur_tests.rs"]
mod tests;
