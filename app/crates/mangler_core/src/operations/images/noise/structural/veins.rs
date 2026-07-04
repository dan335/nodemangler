//! Veins noise image generator.
//!
//! Produces a seamlessly tiling grayscale image of vein/marble patterns: a
//! directional stripe field is domain-warped by multi-octave periodic Perlin
//! fBm, then shaped by a sharpness exponent into thin bright veins on a dark
//! ground. The classic marble construction `sin(k·p + turbulence(p))`, useful
//! for marble, malachite, agate, and other banded minerals.
//!
//! Reuses `periodic_perlin_2d` and `build_perm_tables` for seamless tiling.
//! The stripe direction is snapped to integer cycle counts per axis so the
//! pattern always tiles, even at arbitrary angles.

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
use crate::operations::images::noise::{periodic_perlin_2d, build_perm_tables};

/// Amplitude falloff per octave of the underlying fBm.
const PERSISTENCE: f64 = 0.5;
/// Frequency multiplier between octaves of the underlying fBm.
const LACUNARITY: f64 = 2.0;
/// Maximum phase displacement in radians when warp is 1.0.
const WARP_FACTOR: f64 = 9.0;

/// Periodic Perlin fBm: layers multiple octaves of periodic Perlin noise with
/// decreasing amplitude and increasing frequency. Each octave's frequency is
/// rounded to an integer period for seamless tiling.
/// Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_fbm(u: f64, v: f64, octaves: usize, frequency: f64, hashers: &[PermutationTable]) -> f64 {
    let mut result = 0.0;
    let mut attenuation = PERSISTENCE;
    let mut freq = frequency;

    // Scale factor: 1 / sum(persistence^i for i in 1..=octaves)
    let scale_factor = 1.0 / (1..=octaves).fold(0.0, |acc, i| acc + PERSISTENCE.powi(i as i32));

    for hasher in hashers.iter().take(octaves) {
        // Round frequency to integer period for tiling
        let period = freq.round().max(1.0) as isize;
        let px = u * period as f64;
        let py = v * period as f64;

        let mut signal = periodic_perlin_2d(px, py, period, period, hasher);
        signal *= attenuation;
        attenuation *= PERSISTENCE;
        result += signal;
        freq *= LACUNARITY;
    }

    result * scale_factor
}

/// Operation that generates a seamlessly tiling marble/vein noise image.
///
/// Builds a directional stripe coordinate from an integer-snapped frequency
/// vector, displaces its phase with periodic Perlin fBm (the warp), and shapes
/// `1 - |sin|` of the result with a sharpness exponent. Low sharpness gives
/// broad soft bands; high sharpness gives thin, crisp veins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseVeins {}

impl OpImageNoiseVeins {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "veins noise".to_string(),
            description: "Domain-warped stripe/vein noise. Creates marble, malachite, and agate-style banded vein patterns.".to_string(),
            help: "The classic marble construction: a directional stripe field sin(k.p) has its phase displaced by multi-octave periodic Perlin fBm, bending straight bands into flowing veins. The vein profile is 1 - |sin| raised to a sharpness exponent, so bright veins sit on a dark ground.\n\nVein frequency sets how many stripe cycles cross the tile and angle rotates them; for seamless tiling the rotated frequency vector is snapped to integer cycle counts per axis, so the angle quantizes to the nearest tileable direction. Warp scales how far the fBm bends the stripes: 0 gives ruler-straight bands, 1 gives heavily folded, turbulent veining. Scale and octaves shape the warping fBm itself (blob size and fine detail). Sharpness maps to an exponent between 1 and 16: low values give broad soft bands, high values give thin crisp veins.\n\nIdeal for marble, malachite, agate, wood-free banding, and as a height/roughness source for polished stone materials.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, fBm shape, and stripe controls.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the warping fBm; change to reshape the vein folds."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("scale".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { clamp: Some((1.0, 256.0)), speed: Some(0.1) }), None)
                .with_description("Base lattice cells of the warping fBm; higher values fold the veins at a finer scale."),
            Input::new("octaves".to_string(), Value::Integer(5), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the warp; more octaves add finer wrinkles to the veins."),
            Input::new("vein_frequency".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None)
                .with_description("Number of stripe cycles across the tile; higher values pack veins tighter."),
            Input::new("warp".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How far the fBm bends the stripes; 0 is straight bands, 1 is heavily folded veining."),
            Input::new("sharpness".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Vein profile exponent; low values give broad soft bands, high values give thin crisp veins."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: None, speed: Some(1.0) }), None)
                .with_description("Stripe direction in degrees; snapped to the nearest tileable direction."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of bright marble veins on a dark ground."),
        ]
    }

    /// Generates a veins noise image from the given inputs.
    ///
    /// For each pixel: computes the stripe phase `TAU * (kx*u + ky*v)` from the
    /// integer-snapped frequency vector, adds `warp * fbm(u, v) * WARP_FACTOR`
    /// as a phase displacement, and shapes `1 - |sin(phase)|` with the
    /// sharpness exponent.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let vein_frequency_converted = convert_input(inputs, 5, ValueType::Integer, &mut input_errors);
        let warp_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let sharpness_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Integer(vein_frequency) = vein_frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(warp) = warp_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sharpness) = sharpness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let scale = (scale as f64).max(1.0);
        let octaves = (octaves as usize).clamp(1, 8);
        let vein_frequency = vein_frequency.clamp(1, 64) as f64;
        let warp = (warp as f64).clamp(0.0, 1.0);
        let sharpness = (sharpness as f64).clamp(0.0, 1.0);
        let angle_rad = (angle as f64).to_radians();

        // Snap the rotated frequency vector to integer cycle counts per axis so
        // the stripe field tiles seamlessly at any angle. Since |cos| or |sin|
        // is always >= 1/sqrt(2) and vein_frequency >= 1, at least one component
        // rounds to a nonzero integer; the guard covers pathological input.
        let mut kx = (vein_frequency * angle_rad.cos()).round();
        let ky = (vein_frequency * angle_rad.sin()).round();
        if kx == 0.0 && ky == 0.0 {
            kx = 1.0;
        }

        // Sharpness maps to an exponent between 1 and 16
        let exponent = 1.0 + sharpness * 15.0;

        let perm_tables = build_perm_tables(seed as u32, octaves);
        let perm_ref = &perm_tables;

        let w = width as usize;
        let h = height as usize;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;

                // Stripe phase from the snapped frequency vector, displaced by fBm
                let fbm_value = periodic_fbm(u, v, octaves, scale, perm_ref);
                let s = std::f64::consts::TAU * (kx * u + ky * v) + warp * fbm_value * WARP_FACTOR;

                // Vein profile: bright thin lines where sin crosses zero
                let raw = s.sin();
                let veins = (1.0 - raw.abs()).powf(exponent);

                linear_to_nonlinear_srgb(veins as f32)
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
#[path = "veins_tests.rs"]
mod tests;
