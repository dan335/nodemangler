//! Phasor noise image generator.
//!
//! Produces a grayscale image using phasor noise (Tricard et al. 2019): the
//! same sparse convolution of oriented Gaussian-windowed waves as Gabor noise,
//! but reconstructed from the *phase* of the summed complex field instead of
//! its amplitude. The result is oriented stripes of constant contrast that
//! never wash out - crisp banding where Gabor noise blurs.
//!
//! Ideal for fabric weave, brushed metal anisotropy, dunes, wood pore lines,
//! and any texture needing sharp procedural stripes with organic irregularity.
//!
//! Always tiles seamlessly by wrapping kernel positions at image boundaries.

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

/// Precomputed per-cell impulse: jittered position offset within its cell,
/// random phase shift, and orientation (as cos/sin) of the kernel it drops.
/// These depend only on the cell — never the pixel — so they are derived once
/// before the pixel loop instead of being re-hashed per pixel.
struct CellImpulse {
    jx: f64,
    jy: f64,
    phase: f64,
    cos_a: f64,
    sin_a: f64,
}

/// Operation that generates a phasor noise image.
///
/// Sums Gaussian-windowed complex exponentials (one oriented kernel per grid
/// cell, each with a random phase) into a complex field, then outputs a
/// profile of the field's phase angle. Because the phase carries no amplitude,
/// the stripes keep full contrast everywhere, unlike Gabor noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoisePhasor {}

impl OpImageNoisePhasor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "phasor noise".to_string(),
            description: "Crisp constant-contrast oriented stripes from the phase of a Gabor field. Creates fabric weave, brushed metal, dunes, and wood pore lines.".to_string(),
            help: "Phasor noise is Gabor noise's sharper sibling: the same sparse sum of oriented Gaussian-windowed waves, but the output is the PHASE angle of the summed complex field rather than its amplitude. Phase has no magnitude, so the stripes keep full contrast everywhere instead of washing out where kernels cancel.\n\nOrientation sets a shared stripe direction; random orientation gives each kernel its own angle, producing swirling stripe singularities. Kernel frequency sets stripe density, bandwidth sets kernel overlap (higher is smoother, more coherent stripes), and the sawtooth profile swaps the sine wave for a linear ramp - useful as a gradient field or for hard-edged banding after a threshold.\n\nBest for fabric weave, brushed metal anisotropy, sand dunes, wood pores, and any texture needing crisp procedural stripes with organic wobble.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, orientation, frequency, bandwidth, density, and profile.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for kernel placement and phases."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("orientation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Shared stripe orientation in degrees when random orientation is off."),
            Input::new("random_orientation".to_string(), Value::Bool(false), None, None)
                .with_description("When true each kernel picks its own angle, producing swirling stripe singularities."),
            Input::new("kernel_frequency".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: Some((0.01, 1.0)), speed: Some(0.001) }), None)
                .with_description("Spatial frequency of the wave inside each kernel; higher values pack stripes tighter."),
            Input::new("bandwidth".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: Some((0.1, 10.0)), speed: Some(0.1) }), None)
                .with_description("Gaussian envelope width; larger values overlap more kernels for smoother stripes."),
            Input::new("density".to_string(), Value::Decimal(16.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.5) }), None)
                .with_description("Number of kernels across the image."),
            Input::new("sawtooth".to_string(), Value::Bool(false), None, None)
                .with_description("When true outputs a linear phase ramp instead of a sine wave, giving sawtooth banding."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale phasor noise of crisp constant-contrast stripes."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords, seed, and channel.
    #[inline(always)]
    fn hash(ix: i32, iy: i32, seed: u32, channel: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Generates a phasor noise image from the given inputs.
    ///
    /// For each pixel, sums the Gaussian-windowed complex exponentials of all
    /// nearby kernels into (re, im), takes the phase via atan2, and maps it
    /// through a sine or sawtooth profile to [0, 1].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let orientation_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let random_orient_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);
        let kernel_freq_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let bandwidth_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let density_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let sawtooth_converted = convert_input(inputs, 8, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(orientation) = orientation_converted.unwrap() else { unreachable!() };
        let Value::Bool(random_orientation) = random_orient_converted.unwrap() else { unreachable!() };
        let Value::Decimal(kernel_freq) = kernel_freq_converted.unwrap() else { unreachable!() };
        let Value::Decimal(bandwidth) = bandwidth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Bool(sawtooth) = sawtooth_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let orientation_rad = (orientation as f64).to_radians();
        let kernel_freq = (kernel_freq as f64).clamp(0.01, 1.0);
        let bandwidth = (bandwidth as f64).max(0.1);
        let density = (density as f64).max(1.0);

        // Sigma derived from bandwidth: controls how wide each kernel is
        let sigma = bandwidth / density;
        // Truncation radius: kernels beyond this distance contribute negligibly
        let truncation = 3.0 * sigma;

        let grid_size = density.ceil() as i32;
        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // Precompute every cell's impulse parameters once: position jitter,
        // phase, and orientation are functions of the cell alone.
        let cells = grid_size as usize;
        let mut impulses: Vec<CellImpulse> = Vec::with_capacity(cells * cells);
        for cy in 0..grid_size {
            for cx in 0..grid_size {
                let angle = if random_orientation {
                    Self::hash(cx, cy, seed_u32, 3) * std::f64::consts::TAU
                } else {
                    orientation_rad
                };
                impulses.push(CellImpulse {
                    jx: Self::hash(cx, cy, seed_u32, 0),
                    jy: Self::hash(cx, cy, seed_u32, 1),
                    phase: Self::hash(cx, cy, seed_u32, 2) * std::f64::consts::TAU,
                    cos_a: angle.cos(),
                    sin_a: angle.sin(),
                });
            }
        }
        let impulses_ref = &impulses;

        // Search radius in cells (based on truncation distance in UV space)
        let search = (truncation * density).ceil() as i32 + 1;
        let trunc_sq = truncation * truncation;
        let carrier_freq = kernel_freq * density;
        let inv_two_sigma_sq = 0.5 / (sigma * sigma);

        // For each pixel, accumulate the complex phasor field and take its phase
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position in grid space
                let gx = (px as f64 / w as f64) * density;
                let gy = (py as f64 / h as f64) * density;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                let mut re = 0.0_f64;
                let mut im = 0.0_f64;

                for dy in -search..=search {
                    for dx in -search..=search {
                        let cx = (cell_x + dx).rem_euclid(grid_size);
                        let cy = (cell_y + dy).rem_euclid(grid_size);
                        let impulse = &impulses_ref[cy as usize * cells + cx as usize];

                        // Kernel position within cell (jittered), displacement in UV space
                        let kx = (cell_x + dx) as f64 + impulse.jx;
                        let ky = (cell_y + dy) as f64 + impulse.jy;
                        let disp_x = (gx - kx) / density;
                        let disp_y = (gy - ky) / density;
                        let dist_sq = disp_x * disp_x + disp_y * disp_y;

                        if dist_sq > trunc_sq {
                            continue;
                        }

                        // Gaussian-windowed complex exponential along the kernel's orientation
                        let gaussian = (-dist_sq * inv_two_sigma_sq).exp();
                        let rx = disp_x * impulse.cos_a + disp_y * impulse.sin_a;
                        let arg = std::f64::consts::TAU * carrier_freq * rx + impulse.phase;
                        re += gaussian * arg.cos();
                        im += gaussian * arg.sin();
                    }
                }

                // Phase of the summed field, mapped to [0, 1] by the chosen profile
                let phi = im.atan2(re);
                if sawtooth {
                    phi / std::f64::consts::TAU + 0.5
                } else {
                    phi.sin() * 0.5 + 0.5
                }
            })
        }).collect();

        // No normalization — the phase profile is already in [0, 1]
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let linear = buffer[y * w + x] as f32;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(linear);
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
#[path = "phasor_tests.rs"]
mod tests;
