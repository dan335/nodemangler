//! Hybrid multifractal noise image generator.
//!
//! Produces a seamlessly tiling grayscale image using hybrid multifractal noise,
//! which creates smooth valley bottoms at all altitudes while maintaining fractal
//! detail on ridges and peaks.

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
use crate::operations::images::noise::{periodic_perlin_2d, build_perm_tables};

/// Periodic hybrid multifractal noise: creates smooth valley bottoms while
/// maintaining fractal detail on ridges and peaks. Each octave's frequency is
/// rounded to an integer period for seamless tiling.
/// Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_hybrid_multi(u: f64, v: f64, octaves: usize, frequency: f64, lacunarity: f64, persistence: f64, hashers: &[PermutationTable]) -> f64 {
    let mut freq = frequency;
    let mut attenuation = persistence;

    // Scale factor matching the noise crate's approach
    let scale_factor = {
        let mut result = persistence;
        let mut amplitude = persistence;
        let mut weight = result;
        let mut signal = amplitude;
        weight *= signal;
        result += signal;
        if octaves >= 1 {
            result += (1..=octaves).fold(0.0, |acc, _| {
                amplitude *= persistence;
                weight = weight.max(1.0);
                signal = amplitude;
                weight *= signal;
                acc + signal
            });
        }
        2.0 / result
    };

    // First octave (unscaled, weighted by persistence)
    let period0 = freq.round().max(1.0) as isize;
    let mut result = periodic_perlin_2d(u * period0 as f64, v * period0 as f64, period0, period0, &hashers[0]) * persistence;
    let mut weight = result;

    freq *= lacunarity;

    // Remaining octaves
    for hasher in hashers.iter().take(octaves).skip(1) {
        // Prevent divergence
        weight = weight.max(1.0);

        let period = freq.round().max(1.0) as isize;
        let px = u * period as f64;
        let py = v * period as f64;

        let mut signal = periodic_perlin_2d(px, py, period, period, hasher);
        signal *= attenuation;
        attenuation *= persistence;
        // Add weighted by previous octave's value
        result += weight * signal;
        // Update weight
        weight *= signal;
        freq *= lacunarity;
    }

    result * scale_factor
}

/// Operation that generates a seamlessly tiling grayscale image from hybrid multifractal noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseHybridMultifractalNoise {}

impl OpImageNoiseHybridMultifractalNoise {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hybrid multifractal noise".to_string(),
            description: "Noise function that outputs seamlessly tiling hybrid Multifractal noise. The result of this multifractal noise is that valleys in the noise should have smooth bottoms at all altitudes.".to_string(),
            help: "A multifractal fBm where each octave is weighted by the previous octave's output. Low-signal regions (valleys) accumulate very little extra detail, while high-signal regions (ridges) pick up full fractal roughness. The effect is smooth basins with sharp, detailed peaks.\n\nFrequency sets the base feature size; octaves, lacunarity, and persistence behave like standard fBm. Lower persistence emphasizes the flat valleys; more octaves add more craggy detail on top of the ridges.\n\nIdeal for realistic mountain/valley terrain heightmaps and for weathered rock surfaces where lows should feel quiet.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, octaves, frequency, lacunarity, and persistence.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed controlling where ridges and valleys land."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (0.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of octaves summed; more octaves add detail on ridges while valleys stay smooth."),
            Input::new("frequency".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None)
                .with_description("Base lattice period; higher values produce smaller features."),
            Input::new("lacunarity".to_string(), Value::Decimal(2.094_395_2), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Frequency multiplier between octaves; larger values add finer detail faster."),
            Input::new("persistence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Amplitude falloff per octave; lower values emphasize the smooth valleys."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Seamlessly tiling grayscale hybrid multifractal noise image."),
        ]
    }

    /// Generates a hybrid multifractal noise image from the given inputs.
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
                // Lattice-periodic hybrid multifractal: each octave uses an integer period
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let noise = periodic_hybrid_multi(u, v, oct, freq, lacunarity as f64, persistence as f64, perm_ref) as f32 * 0.5 + 0.5;
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
#[path = "hybrid_multifractal_tests.rs"]
mod tests;
