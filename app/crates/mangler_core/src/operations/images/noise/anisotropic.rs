//! Anisotropic noise image generator.
//!
//! Produces a seamlessly tiling grayscale image with directionally biased noise.
//! Stretches periodic Perlin noise along a configurable angle and ratio, creating
//! elongated patterns useful for brushed metal, wood grain, or fabric textures.
//! Reuses `periodic_perlin_2d` and `build_perm_tables` for seamless tiling.

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
use super::{periodic_perlin_2d, build_perm_tables};

/// Periodic anisotropic fBm noise: layers periodic Perlin noise octaves with
/// directional stretching applied by rotating coordinates, scaling along the
/// stretch axis, then rotating back. Each octave uses an integer period for tiling.
/// Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_anisotropic_fbm(
    u: f64, v: f64,
    octaves: usize, frequency: f64, lacunarity: f64, persistence: f64,
    angle_rad: f64, stretch: f64,
    hashers: &[PermutationTable],
) -> f64 {
    // Precompute rotation for coordinate stretching
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    let mut result = 0.0;
    let mut attenuation = persistence;
    let mut freq = frequency;

    let scale_factor = 1.0 / (1..=octaves).fold(0.0, |acc, i| acc + persistence.powi(i as i32));

    for i in 0..octaves {
        let period = freq.round().max(1.0) as isize;
        // Stretch period along the perpendicular axis to keep tiling correct
        let period_stretch = (freq * stretch).round().max(1.0) as isize;

        // Scale coordinates to lattice space
        let su = u * period as f64;
        let sv = v * period_stretch as f64;

        // Rotate into stretch space, apply stretch, rotate back
        let ru = su * cos_a + sv * sin_a;
        let rv = -su * sin_a + sv * cos_a;

        // Use stretched periods for each axis to maintain tiling
        let mut signal = periodic_perlin_2d(ru, rv, period, period_stretch, &hashers[i]);
        signal *= attenuation;
        attenuation *= persistence;
        result += signal;
        freq *= lacunarity;
    }

    result * scale_factor
}

/// Operation that generates a seamlessly tiling anisotropic noise image.
///
/// Configurable via angle (stretch direction), stretch ratio, octaves, frequency,
/// lacunarity, and persistence. Higher stretch ratios produce more elongated patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseAnisotropic {}

impl OpImageNoiseAnisotropic {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "anisotropic noise".to_string(),
            description: "Creates a seamlessly tiling image with directionally stretched noise patterns.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, angle, stretch, octaves, frequency, lacunarity, and persistence.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("stretch".to_string(), Value::Decimal(4.0), Some(InputSettings::Slider { range: (1.0, 20.0), step_by: Some(0.1), clamp_to_range: false }), None),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("frequency".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None),
            Input::new("lacunarity".to_string(), Value::Decimal(2.094_395_2), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None),
            Input::new("persistence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates an anisotropic noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let angle_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let stretch_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 5, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);
        let lacunarity_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let persistence_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(stretch) = stretch_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Integer(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lacunarity) = lacunarity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(persistence) = persistence_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let freq = frequency.max(1) as f64;
        let oct = octaves.max(1) as usize;
        let angle_rad = (angle as f64).to_radians();
        let stretch_val = (stretch as f64).max(1.0);

        let perm_tables = build_perm_tables(seed as u32, oct);
        let perm_ref = &perm_tables;

        let w = width as usize;
        let h = height as usize;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let noise = periodic_anisotropic_fbm(
                    u, v, oct, freq, lacunarity as f64, persistence as f64,
                    angle_rad, stretch_val, perm_ref,
                ) as f32 * 0.5 + 0.5;
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
#[path = "anisotropic_tests.rs"]
mod tests;
