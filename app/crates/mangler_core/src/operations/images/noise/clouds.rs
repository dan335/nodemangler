//! Cloud noise image generator.
//!
//! Produces a seamlessly tiling grayscale image with soft, billowy cloud patterns.
//! Uses multi-octave periodic value noise (fBm with value noise instead of Perlin),
//! which produces smoother, rounder blobs compared to Perlin-based fBm — ideal for
//! cloud and fog textures. Reuses `periodic_value_2d` and `build_perm_tables`.

use rayon::prelude::*;
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
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
use noise::permutationtable::PermutationTable;
use super::{periodic_value_2d, build_perm_tables};

/// Periodic cloud noise: layers multiple octaves of periodic value noise with
/// decreasing amplitude and increasing frequency. Each octave's frequency is
/// rounded to an integer period for seamless tiling.
/// Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_cloud_fbm(u: f64, v: f64, octaves: usize, frequency: f64, lacunarity: f64, persistence: f64, hashers: &[PermutationTable]) -> f64 {
    let mut result = 0.0;
    let mut attenuation = persistence;
    let mut freq = frequency;

    // Scale factor: 1 / sum(persistence^i for i in 1..=octaves)
    let scale_factor = 1.0 / (1..=octaves).fold(0.0, |acc, i| acc + persistence.powi(i as i32));

    for hasher in hashers.iter().take(octaves) {
        // Round frequency to integer period for tiling
        let period = freq.round().max(1.0) as isize;
        let px = u * period as f64;
        let py = v * period as f64;

        let mut signal = periodic_value_2d(px, py, period, period, hasher);
        signal *= attenuation;
        attenuation *= persistence;
        result += signal;
        freq *= lacunarity;
    }

    result * scale_factor
}

/// Operation that generates a seamlessly tiling cloud noise image.
///
/// Configurable via octaves, frequency, lacunarity, and persistence.
/// Uses value noise as the base function for soft, rounded cloud shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseClouds {}

impl OpImageNoiseClouds {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cloud noise".to_string(),
            description: "Creates a seamlessly tiling image with soft cloud-like patterns using multi-octave value noise.".to_string(),
            help: "fBm built on value noise instead of Perlin gradient noise. Value noise interpolates between random lattice values, which gives rounder, blobbier features than Perlin's directional gradients, so the layered result looks softer and more fluffy.\n\nFrequency sets the base blob size, octaves stack finer wispy detail on top, lacunarity is the per-octave frequency multiplier, and persistence is the amplitude falloff. Lower persistence softens the result; higher lacunarity adds wispy filaments faster.\n\nIdeal for clouds, fog, smoke, and watercolor-style textures.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, octaves, frequency, lacunarity, and persistence.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the cloud pattern; change to get a different cloudscape."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of value-noise octaves summed; higher values add wispy detail."),
            Input::new("frequency".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None)
                .with_description("Base lattice period; higher values produce smaller, denser cloud blobs."),
            Input::new("lacunarity".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Frequency multiplier between octaves; higher values add wispier detail."),
            Input::new("persistence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Amplitude falloff per octave; lower values give smoother, softer clouds."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image with soft, rounded cloud shapes."),
        ]
    }

    /// Generates a cloud noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let octaves_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let lacunarity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let persistence_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Integer(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lacunarity) = lacunarity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(persistence) = persistence_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let freq = frequency.max(1) as f64;
        let oct = octaves.max(1) as usize;

        let perm_tables = build_perm_tables(seed as u32, oct);
        let perm_ref = &perm_tables;

        let w = width as usize;
        let h = height as usize;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let noise = periodic_cloud_fbm(u, v, oct, freq, lacunarity as f64, persistence as f64, perm_ref) as f32 * 0.5 + 0.5;
                linear_to_nonlinear_srgb(noise)
            })
        }).collect();

        // Build a single-channel FloatImage from the computed pixel values
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for (i, &val) in pixels.iter().enumerate() {
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            float_image.put_pixel(x, y, &[val]);
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "clouds_tests.rs"]
mod tests;
