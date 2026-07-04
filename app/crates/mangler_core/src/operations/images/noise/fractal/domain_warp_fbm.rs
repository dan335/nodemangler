//! Domain-warped fBm noise image generator.
//!
//! Produces organic, grunge-like textures by recursively warping the coordinate
//! space with fBm noise before the final sample. Each warp iteration distorts
//! the coordinates using a separate fBm evaluation with unique offsets, creating
//! flowing, paint-like smears and stains. Based on Inigo Quilez's domain warping
//! technique: `f(p + f(p + f(p)))`.
//!
//! Always tiles seamlessly via 4D torus mapping.

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
use noise::{NoiseFn, MultiFractal, Perlin, Fbm};

/// Operation that generates a grayscale image from domain-warped fBm noise.
///
/// Applies recursive domain warping to fractal Brownian motion noise, producing
/// organic, grunge-like textures with flowing distortions. The `warp_iterations`
/// parameter controls the recursion depth (1 = `f(p + f(p))`, 2 = `f(p + f(p + f(p)))`),
/// and `warp_strength` controls the amplitude of each warp displacement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseDomainWarpFbm {}

impl OpImageNoiseDomainWarpFbm {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "domain warp".to_string(),
            description: "Domain-warped fBm noise. Recursively distorts coordinates with fBm to produce organic, grunge-like textures.".to_string(),
            help: "Inigo Quilez-style recursive domain warping: instead of sampling fBm at (x, y), the coordinates themselves are displaced by another fBm sample before the final lookup, following the pattern f(p + f(p + f(p))). Each warp iteration uses unique offsets so the layers decorrelate.\n\nOctaves/frequency/lacunarity/persistence shape the underlying fBm. Warp iterations set the recursion depth; warp strength scales how far coordinates are pushed. Higher strength and more iterations smear the pattern into paint-like streaks.\n\nIdeal for marble veining, flowing magma, ink bleeds, and any organic, turbulent look.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, fractal params, and warp controls.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the warped fBm; change to reshape the smears and stains."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            // octaves must be ≥ 1 — Fbm with 0 octaves returns NaN which then
            // poisons the warp loop and crashes the noise crate's floor_to_isize.
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the underlying noise; more adds finer detail inside the warp."),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Base frequency of the fBm; higher values produce smaller flowing features."),
            Input::new("lacunarity".to_string(), Value::Decimal(2.094_395_2), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Frequency multiplier between octaves of the fBm layer."),
            Input::new("persistence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Amplitude falloff per octave of the fBm layer."),
            Input::new("warp_iterations".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 4.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of recursive warp passes; more iterations produce deeper, more chaotic distortion."),
            Input::new("warp_strength".to_string(), Value::Decimal(0.8), Some(InputSettings::DragValue { clamp: Some((0.0, 10.0)), speed: Some(0.01) }), None)
                .with_description("Amplitude of each domain warp; higher values smear the pattern further."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of domain-warped fBm with paint-like smears."),
        ]
    }

    /// Samples fBm noise at the given 4D coordinates.
    ///
    /// Returns a value roughly in [-1, 1] range. Guards against non-finite
    /// inputs because the underlying noise crate's `floor_to_isize` panics on
    /// NaN/Inf coordinates — and the warp loop can produce one bad iteration
    /// that would poison every later iteration without this check.
    fn sample_fbm(fbm: &Fbm<Perlin>, coords: [f64; 4]) -> f64 {
        if !coords.iter().all(|c| c.is_finite()) {
            return 0.0;
        }
        let v = fbm.get(coords);
        if v.is_finite() { v } else { 0.0 }
    }

    /// Computes 4D torus coordinates from normalized (u, v) in [0, 1].
    ///
    /// Maps 2D coordinates onto a 4D torus surface so that the noise tiles seamlessly
    /// at all edges. The radius `r` is `1 / TAU` to keep scale consistent with the
    /// noise's internal frequency.
    fn torus_coords(u: f64, v: f64) -> [f64; 4] {
        let tau = std::f64::consts::TAU;
        let r = 1.0 / tau;
        [
            (tau * u).cos() * r,
            (tau * u).sin() * r,
            (tau * v).cos() * r,
            (tau * v).sin() * r,
        ]
    }

    /// Generates a domain-warped fBm noise image from the given inputs.
    ///
    /// For each pixel, the algorithm:
    /// 1. Computes torus-mapped base coordinates for seamless tiling
    /// 2. Applies `warp_iterations` layers of domain warping, where each layer
    ///    offsets the coordinates by sampling fBm with unique constant offsets
    /// 3. Performs a final fBm sample at the warped coordinates
    /// 4. Normalizes and gamma-corrects the result
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let octaves_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let lacunarity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let persistence_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let warp_iterations_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);
        let warp_strength_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        // Return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Get values
        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lacunarity) = lacunarity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(persistence) = persistence_converted.unwrap() else { unreachable!() };
        let Value::Integer(warp_iterations) = warp_iterations_converted.unwrap() else { unreachable!() };
        let Value::Decimal(warp_strength) = warp_strength_converted.unwrap() else { unreachable!() };

        // Clamp values
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let warp_iterations = warp_iterations.clamp(1, 4) as usize;
        let warp_strength = warp_strength as f64;
        // Guard against octaves=0 even if a saved graph predates the slider min:
        // Fbm::get with 0 octaves returns NaN, which propagates through the warp
        // loop and crashes the noise crate's floor_to_isize.
        let octaves = octaves.max(1);

        // Create the fBm noise generator
        let fbm = Fbm::<Perlin>::new(seed as u32)
            .set_frequency(frequency as f64)
            .set_octaves(octaves as usize)
            .set_lacunarity(lacunarity as f64)
            .set_persistence(persistence as f64);

        // Large prime-based offsets to decorrelate each warp layer's X and Y samples.
        let offsets: [(f64, f64, f64, f64); 4] = [
            (1.7, 9.2, 8.3, 2.8),
            (5.2, 1.3, 3.7, 7.1),
            (2.1, 6.8, 4.5, 9.7),
            (7.4, 3.9, 6.2, 1.5),
        ];

        let w = width as usize;
        let h = height as usize;

        // Compute all pixels in parallel using torus-mapped coordinates for seamless tiling.
        let fbm_ref = &fbm;
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                // Compute torus-mapped base coordinates
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;

                // Apply domain warping iteratively.
                let mut warp_x = 0.0;
                let mut warp_y = 0.0;

                for &(ox1, oy1, ox2, oy2) in offsets.iter().take(warp_iterations) {

                    let warp_coords = Self::torus_coords(u + warp_x, v + warp_y);
                    warp_x = Self::sample_fbm(fbm_ref, [
                        warp_coords[0] + ox1, warp_coords[1] + oy1,
                        warp_coords[2] + ox1, warp_coords[3] + oy1,
                    ]) * warp_strength;
                    warp_y = Self::sample_fbm(fbm_ref, [
                        warp_coords[0] + ox2, warp_coords[1] + oy2,
                        warp_coords[2] + ox2, warp_coords[3] + oy2,
                    ]) * warp_strength;
                }

                // Final sample at warped torus coordinates
                let final_coords = Self::torus_coords(u + warp_x, v + warp_y);
                let noise = Self::sample_fbm(fbm_ref, final_coords) as f32 * 0.5 + 0.5;

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
#[path = "domain_warp_fbm_tests.rs"]
mod tests;
