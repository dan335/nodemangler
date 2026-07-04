//! Fault terrain image generator.
//!
//! Produces a seamlessly tiling grayscale heightmap using fault formation:
//! the terrain starts flat and hundreds of random fault lines each raise one
//! side and lower the other, with later faults displacing less. The
//! accumulated steps build the characteristic large-scale ridges, plateaus,
//! and escarpments of fault-block terrain that fractal noise lacks.
//!
//! Tiling is preserved by using periodic faults: each fault is the sign of a
//! sinusoid with an integer wave vector, so every fault (and therefore the
//! sum) wraps exactly at the image edges.

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

/// One precomputed fault: an integer wave vector (for periodicity), a random
/// phase, and the displacement amplitude for this iteration.
struct Fault {
    a: f64,
    b: f64,
    phase: f64,
    amplitude: f64,
}

/// Operation that generates a fault-formation terrain heightmap.
///
/// Precomputes `iterations` periodic faults with linearly decreasing
/// amplitude, then per pixel sums each fault's displacement: a smoothed sign
/// of `sin(TAU * (a*u + b*v) + phase)`. The result is min/max normalized to
/// the full [0, 1] range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseFaultTerrain {}

impl OpImageNoiseFaultTerrain {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "fault terrain".to_string(),
            description: "Terrain heightmap built from hundreds of random fault lines. Creates ridges, plateaus, and escarpments that fractal noise lacks.".to_string(),
            help: "Classic fault-formation terrain: the surface starts flat, then each iteration picks a random fault and raises one side while lowering the other. Early faults displace the most and later ones progressively less (set by falloff), so large landmasses form first and detail accumulates on top. To keep the tile seamless each fault is periodic - the smoothed sign of a random sinusoid - rather than a straight line.\n\nFrequency caps the fault wave vectors: 1-2 gives a few continental ridges, higher values fracture the terrain finer. Smoothness softens the fault steps from sheer cliffs (0) to rolling hills (1). More iterations give smoother, more natural accumulation.\n\nOutput is min/max normalized to [0, 1]. Pairs well with the erosion, normal from height, and ao from height nodes for full terrain workflows.".to_string(),
        }
    }

    /// Creates the default inputs for the fault terrain operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the fault sequence; change for different terrain."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("iterations".to_string(), Value::Integer(256), Some(InputSettings::DragValue { clamp: Some((1.0, 2000.0)), speed: None }), None)
                .with_description("Number of fault lines applied; more iterations accumulate smoother terrain."),
            Input::new("frequency".to_string(), Value::Integer(3), Some(InputSettings::DragValue { clamp: Some((1.0, 16.0)), speed: None }), None)
                .with_description("Maximum fault wave frequency; low values make continental ridges, high values fracture finer."),
            Input::new("smoothness".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Softens fault steps; 0 gives sheer cliffs, 1 gives rolling sinusoidal hills."),
            Input::new("falloff".to_string(), Value::Decimal(0.9), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much fault displacement shrinks over the iterations; higher values emphasize early large faults."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale fault-formation terrain heightmap normalized to [0, 1]."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from an iteration index, seed, and channel.
    #[inline(always)]
    fn hash(i: u32, seed: u32, channel: u32) -> f64 {
        let mut h = i.wrapping_mul(1597334677)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Generates a fault terrain heightmap image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let iterations_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let smoothness_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let falloff_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(iterations) = iterations_converted.unwrap() else { unreachable!() };
        let Value::Integer(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(smoothness) = smoothness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(falloff) = falloff_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let iterations = iterations.clamp(1, 2000) as usize;
        let max_freq = frequency.clamp(1, 16);
        let smoothness = (smoothness as f64).clamp(0.0, 1.0);
        let falloff = (falloff as f64).clamp(0.0, 1.0);

        let seed_u32 = seed as u32;

        // Precompute all faults. Wave vector components are integers so each
        // sinusoid — and therefore the whole sum — wraps exactly at the tile edge.
        let faults: Vec<Fault> = (0..iterations).map(|i| {
            let iu = i as u32;
            // Random integer wave vector in [-max_freq, max_freq]^2, excluding (0, 0)
            let span = (2 * max_freq + 1) as f64;
            let mut a = (Self::hash(iu, seed_u32, 0) * span).floor() as i32 - max_freq;
            let mut b = (Self::hash(iu, seed_u32, 1) * span).floor() as i32 - max_freq;
            if a == 0 && b == 0 {
                // Degenerate direction: fall back to a diagonal fault
                a = 1;
                b = 1;
            }
            // Linearly decreasing displacement: from 1 down to (1 - falloff)
            let t = if iterations > 1 { i as f64 / (iterations - 1) as f64 } else { 0.0 };
            Fault {
                a: a.clamp(-16, 16) as f64,
                b: b.clamp(-16, 16) as f64,
                phase: Self::hash(iu, seed_u32, 2) * std::f64::consts::TAU,
                amplitude: 1.0 - falloff * t,
            }
        }).collect();
        let faults_ref = &faults;

        // Smoothness maps to the transition width of the fault step: at 0 the
        // raw sign is used (cliff), at 1 the full sinusoid passes through (hills).
        let step_width = (smoothness * smoothness).max(1e-4);

        let w = width as usize;
        let h = height as usize;
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let u = px as f64 / w as f64;
                let v = py as f64 / h as f64;

                let mut height_val = 0.0_f64;
                for fault in faults_ref {
                    let s = (std::f64::consts::TAU * (fault.a * u + fault.b * v) + fault.phase).sin();
                    // Smoothed sign: clamp(s / step_width) is a hard step for
                    // small widths and passes the sinusoid through at width 1.
                    height_val += fault.amplitude * (s / step_width).clamp(-1.0, 1.0);
                }
                height_val
            })
        }).collect();

        // Normalize to [0, 1]
        let min_val = buffer.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = buffer.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max_val - min_val).max(1e-10);

        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let normalized = ((buffer[y * w + x] - min_val) / range) as f32;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(normalized);
                float_image.put_pixel(x as u32, y as u32, &[non_linear]);
            }
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
#[path = "fault_terrain_tests.rs"]
mod tests;
