//! Gaussian (white) noise image generator.
//!
//! Produces a seamlessly tiling grayscale image of pseudo-random per-pixel noise.
//! Uses integer hashing of pixel coordinates with periodic wrapping, so the
//! pattern repeats every `scale` pixels in each axis for seamless tiling.

use image::{ImageBuffer, DynamicImage};
use rayon::prelude::*;
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Hash two coordinates and a seed into a pseudo-random value in [0, 1].
///
/// Uses wrapping multiply and XOR-shift mixing for fast, uniform distribution.
#[inline(always)]
fn pixel_hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(1597334677)
        ^ y.wrapping_mul(2943785939)
        ^ seed.wrapping_mul(1013904223);
    h = h.wrapping_mul(h ^ (h >> 16));
    h = h.wrapping_mul(h ^ (h >> 16));
    (h & 0x00FFFFFF) as f32 / 0x01000000 as f32
}

/// Operation that generates a seamlessly tiling grayscale image of white noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseGaussian {}

impl OpImageNoiseGaussian {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gaussian noise".to_string(),
            description: "Creates a seamlessly tiling image of per-pixel white noise.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, and scale (tile period in pixels).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("scale".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates a white noise image from the given inputs.
    ///
    /// Each pixel is independently hashed from its wrapped coordinates and seed,
    /// producing uniform random brightness. Wrapping at `scale` ensures tiling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(scale) = scale_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let period = scale.max(1) as u32;
        let seed_u32 = seed as u32;

        let w = width as usize;
        let h = height as usize;

        // Each pixel is hashed independently; wrap coordinates at `period` for tiling
        let pixels: Vec<u16> = (0..h).into_par_iter().flat_map_iter(|y| {
            (0..w).map(move |x| {
                let wx = (x as u32) % period;
                let wy = (y as u32) % period;
                let noise = pixel_hash(wx, wy, seed_u32);
                let non_linear = linear_to_nonlinear_srgb(noise);
                (non_linear * 65535.0) as u16
            })
        }).collect();

        let image_buffer = ImageBuffer::from_raw(width as u32, height as u32, pixels).unwrap();
        let dynamic_image = DynamicImage::ImageLuma16(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "gaussian_tests.rs"]
mod tests;
