//! Spectral terrain image generator.
//!
//! Produces a seamlessly tiling grayscale heightmap using random-phase
//! spectral synthesis: every Fourier mode up to a wave-vector cutoff gets a
//! power-law amplitude and a random phase, and the surface is the sum of
//! those modes. Unlike fault formation or
//! fractal-sum noises, the power spectrum here is exact and directly
//! controls how rolling or rough the terrain reads.
//!
//! Tiling is preserved because every wave vector is an integer pair, so
//! each cosine term (and therefore the sum) wraps exactly at the image
//! edges.

use rayon::prelude::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::noise::voronoi_common::cell_hash;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// One precomputed Fourier mode: amplitude, phase, the integer y wave
/// number, and the per-pixel row rotation step along x.
struct SpectralMode {
    amplitude: f64,
    phi: f64,
    ky: i32,
    cos_delta: f64,
    sin_delta: f64,
}

/// Operation that generates a heightmap via random-phase spectral synthesis.
///
/// Precomputes every integer wave vector within a circular cutoff, assigns
/// each a `|k|^(-beta/2)` power-law amplitude and a uniform-random phase,
/// then sums the resulting cosines per pixel.
/// The result is min/max normalized to the full [0, 1] range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseSpectralTerrain {}

impl OpImageNoiseSpectralTerrain {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "spectral terrain".to_string(),
            description: "Heightmap from random-phase spectral synthesis: exact power-spectrum control over how rolling or rough the terrain is.".to_string(),
            help: "Random-phase spectral synthesis of a fractional Brownian surface (after Voss 1985; Saupe, 'The Science of Fractal Images', 1988): each Fourier mode gets a |k|^(-beta/2) power-law amplitude and a random phase; beta = 2H+2. Amplitudes are deterministic rather than Rayleigh-random so every seed has the same overall character - only the arrangement changes.\n\nDetail sets the maximum wave-vector component included (a circular cutoff in frequency space); higher values add finer bumps on top of the large-scale shape. Roll off is the spectral exponent beta: around 2 gives rough, Brownian-looking terrain, 3-4 gives rolling hills, and 5 gives very smooth, gentle swells.\n\nTiles exactly because all wave vectors are integers. Deterministic from seed. Output is min/max normalized to [0, 1]. Pairs well with the erosion, normal from height, and ao from height nodes for full terrain workflows.".to_string(),
        }
    }

    /// Creates the default inputs for the spectral terrain operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the mode phases and amplitudes; change for different terrain."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("detail".to_string(), Value::Integer(16), Some(InputSettings::Slider { range: (2.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Maximum wave-vector component included; higher values add finer bumps."),
            Input::new("roll off".to_string(), Value::Decimal(3.5), Some(InputSettings::Slider { range: (1.5, 5.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Power-spectrum exponent beta; higher values make smoother, more rolling terrain."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale spectral-synthesis terrain heightmap normalized to [0, 1]."),
        ]
    }

    /// Generates a spectral terrain heightmap image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let detail_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let roll_off_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(detail) = detail_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roll_off) = roll_off_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let k = detail.clamp(2, 32);
        let beta = (roll_off as f64).clamp(1.5, 5.0);

        let seed_u32 = seed as u32;
        let w = width as usize;
        let h = height as usize;

        // Precompute every mode within the circular wave-vector cutoff. Only
        // the ky >= 0 half-plane is kept for kx == 0 since cosine is even in
        // ky there, which would otherwise double-count that axis.
        let mut modes: Vec<SpectralMode> = Vec::new();
        for kx in 0..=k {
            for ky in -k..=k {
                if kx == 0 && ky == 0 { continue; }
                if kx == 0 && ky < 0 { continue; }
                if kx * kx + ky * ky > k * k { continue; }

                // Deterministic power-law amplitude; randomness lives in the
                // phases only. A Rayleigh-random magnitude (the textbook fBm
                // construction) lets a single low-frequency mode dominate the
                // whole surface on unlucky seeds, so looks vary wildly.
                let r = ((kx * kx + ky * ky) as f64).sqrt();
                let amplitude = r.powf(-beta / 2.0);
                let phi = std::f64::consts::TAU * cell_hash(kx, ky, seed_u32, 1);

                let delta = std::f64::consts::TAU * kx as f64 / w as f64;
                let (sin_delta, cos_delta) = delta.sin_cos();

                modes.push(SpectralMode { amplitude, phi, ky, cos_delta, sin_delta });
            }
        }
        let modes_ref = &modes;

        // Evaluate each row with a rotation recurrence instead of a per-pixel
        // cosine: the per-pixel phase step is constant along a row, so the
        // running (sin, cos) pair can be rotated by a fixed angle each step.
        let rows: Vec<Vec<f64>> = (0..h).into_par_iter().map(move |py| {
            let v = py as f64 / h as f64;
            let mut row = vec![0.0_f64; w];
            for mode in modes_ref {
                let theta0 = std::f64::consts::TAU * mode.ky as f64 * v + mode.phi;
                let (mut s, mut c) = theta0.sin_cos();
                for px in 0..w {
                    row[px] += mode.amplitude * c;
                    let next_c = c * mode.cos_delta - s * mode.sin_delta;
                    let next_s = s * mode.cos_delta + c * mode.sin_delta;
                    c = next_c;
                    s = next_s;
                }
            }
            row
        }).collect();
        let buffer: Vec<f64> = rows.into_iter().flatten().collect();

        // Normalize to [0, 1]
        let min_val = buffer.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = buffer.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_val - min_val;

        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let normalized = if range < 1e-12 {
                    0.5
                } else {
                    ((buffer[y * w + x] - min_val) / range) as f32
                };
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
#[path = "spectral_terrain_tests.rs"]
mod tests;
