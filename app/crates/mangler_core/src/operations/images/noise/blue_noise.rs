//! Blue noise generator (high-passed white noise approximation).
//!
//! Hashes per-pixel white noise, subtracts a wrap-around box-blurred copy to
//! remove the low-frequency content, and renormalizes. The result is spectrally
//! "blue" — energy concentrated in the high frequencies — which dithers and
//! stipples far more pleasingly than white noise. Tiles seamlessly.

use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use super::pixel_hash;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Separable box blur with wrap-around edges (keeps the result tileable).
fn box_blur_wrap(src: &[f32], w: usize, h: usize, r: i32) -> Vec<f32> {
    let mut tmp = vec![0.0f32; w * h];
    let count = (2 * r + 1) as f32;
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for dx in -r..=r {
                let xx = (x as i32 + dx).rem_euclid(w as i32) as usize;
                sum += src[y * w + xx];
            }
            tmp[y * w + x] = sum / count;
        }
    }
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for dy in -r..=r {
                let yy = (y as i32 + dy).rem_euclid(h as i32) as usize;
                sum += tmp[yy * w + x];
            }
            out[y * w + x] = sum / count;
        }
    }
    out
}

/// Operation that generates seamlessly tiling blue-ish noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseBlue {}

impl OpImageNoiseBlue {
    /// Returns the node metadata (name and description) for blue noise.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blue noise".to_string(),
            description: "Creates seamlessly tiling blue noise (high-passed white noise).".to_string(),
            help: "Generates per-pixel white noise by hashing wrapped coordinates, blurs a copy with a wrap-around box filter of the given radius, and subtracts it to strip the low frequencies. The high-pass residual is renormalized to [0,1], leaving noise whose energy sits in the high frequencies — neighbouring samples repel rather than clump.\n\nLarger radius removes more low-frequency content (bluer spectrum). This is an efficient approximation, not a void-and-cluster pattern, but it dithers, stipples, and breaks up banding far better than white noise. Output is a single-channel grayscale image; it tiles seamlessly.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, and high-pass radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the underlying white noise."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("High-pass blur radius; larger values produce a bluer spectrum."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale blue noise image."),
        ]
    }

    /// Generates the blue noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let radius_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let r = radius.max(1);
        let seed_u32 = seed as u32;
        let w = width as usize;
        let h = height as usize;

        // White noise, wrapped per axis so the field is tileable.
        let white: Vec<f32> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| pixel_hash(x as u32, y as u32, seed_u32))
            .collect();

        let blurred = box_blur_wrap(&white, w, h, r);
        // High-pass = white minus its low frequencies.
        let hp: Vec<f32> = white.iter().zip(blurred.iter()).map(|(a, b)| a - b).collect();

        // Renormalize the residual to fill [0, 1].
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &v in &hp {
            min = min.min(v);
            max = max.max(v);
        }
        let range = max - min;

        let mut img = FloatImage::new(width as u32, height as u32, 1);
        for (i, &v) in hp.iter().enumerate() {
            let n = if range > 1e-9 { (v - min) / range } else { 0.5 };
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            img.put_pixel(x, y, &[linear_to_nonlinear_srgb(n)]);
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(img), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "blue_noise_tests.rs"]
mod tests;
