//! Billow noise image generator.
//!
//! Produces a seamlessly tiling grayscale image using billow noise, a fractal noise
//! variant that takes the absolute value of each octave before summing, creating a
//! puffy, cloud-like appearance with soft ridges.

use rayon::prelude::*;
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
use super::{periodic_perlin_2d, build_perm_tables};

/// Periodic billow noise: layers multiple octaves of periodic Perlin noise,
/// taking the absolute value of each octave to create puffy, cloud-like ridges.
/// Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_billow(u: f64, v: f64, octaves: usize, frequency: f64, lacunarity: f64, persistence: f64, hashers: &[PermutationTable]) -> f64 {
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

        let mut signal = periodic_perlin_2d(px, py, period, period, hasher);
        // Take absolute value, scale and shift to [-1, 1] (matches noise crate's scale_shift)
        signal = signal.abs() * 2.0 - 1.0;
        signal *= attenuation;
        attenuation *= persistence;
        result += signal;
        freq *= lacunarity;
    }

    result * scale_factor
}

/// Operation that generates a seamlessly tiling grayscale image from billow noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseBillow {}

impl OpImageNoiseBillow {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "billow noise".to_string(),
            description: "Creates a seamlessly tiling billowy noise image.".to_string(),
            help: "A variant of fBm where each Perlin octave is folded with abs() before being summed, flipping negative regions positive. This turns the usual zero-crossings into soft ridges and produces a puffy, cloud-like look.\n\nOctaves control how many billow layers are added, lacunarity sets the per-octave frequency multiplier, and persistence sets the amplitude falloff. Lower persistence gives softer puffs; higher lacunarity packs finer puffs inside the large ones.\n\nGood for clouds, cotton, smoke, and soft foamy textures.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, octaves, frequency, lacunarity, and persistence.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the billow pattern; change to get a different puff arrangement."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (0.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of billow layers summed; more octaves add finer puff detail."),
            Input::new("frequency".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None)
                .with_description("Base lattice period; higher values create smaller puffs."),
            Input::new("lacunarity".to_string(), Value::Decimal(2.094_395_2), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Frequency multiplier between octaves; higher values add finer puffs faster."),
            Input::new("persistence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Amplitude falloff per octave; lower values produce softer, cloudier puffs."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Seamlessly tiling grayscale billow noise image with soft puffy ridges."),
        ]
    }

    /// Generates a billow noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let octaves_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let lacunarity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let persistence_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Integer(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lacunarity) = lacunarity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(persistence) = persistence_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let freq = frequency.max(1) as f64;

        // Build per-octave permutation tables for periodic tiling.
        // Clamp to [1, 32] (matches the octaves slider's declared range in
        // create_inputs()) so a connected value bypassing the UI slider clamp
        // (e.g. -1, which would otherwise wrap to usize::MAX on cast) can't
        // make build_perm_tables allocate an astronomical number of tables.
        let oct = octaves.clamp(1, 32) as usize;
        let perm_tables = build_perm_tables(seed as u32, oct);
        let perm_ref = &perm_tables;

        let w = width as usize;
        let h = height as usize;
        // Compute pixels in parallel using rayon, iterating rows then columns for correct row-major order.
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                // Lattice-periodic billow: each octave uses an integer period for seamless tiling
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let noise = periodic_billow(u, v, oct, freq, lacunarity as f64, persistence as f64, perm_ref) as f32 * 0.5 + 0.5;
                crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(noise)
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
#[path = "billow_tests.rs"]
mod tests;
