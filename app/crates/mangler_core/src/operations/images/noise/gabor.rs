//! Gabor noise image generator.
//!
//! Produces a grayscale image using sparse convolution of oriented Gabor kernels.
//! Gabor noise excels at creating directional, anisotropic textures such as
//! scratches, brushed metal, wood grain, and streaks that are impossible to
//! achieve with standard isotropic noise functions.
//!
//! Each kernel is a sinusoidal wave modulated by a Gaussian envelope, placed at
//! pseudo-random positions across the image. The kernel orientation, frequency,
//! and bandwidth control the resulting texture character.
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

/// Operation that generates a Gabor noise image.
///
/// Places oriented Gabor kernels (sinusoidal waves with Gaussian envelopes) at
/// pseudo-random positions across the image. The `orientation` controls the
/// direction of the wave pattern, `kernel_frequency` controls the wave density
/// within each kernel, and `bandwidth` controls the kernel's spatial extent.
/// Setting `random_orientation` to true randomizes each kernel's angle for
/// isotropic noise; when false, all kernels share the same orientation for
/// directional textures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseGabor {}

impl OpImageNoiseGabor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gabor noise".to_string(),
            description: "Directional noise using oriented Gabor kernels. Creates scratches, brushed metal, and wood grain textures.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, orientation, frequency, bandwidth, and density.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None),
            Input::new("orientation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("random_orientation".to_string(), Value::Bool(false), None, None),
            Input::new("kernel_frequency".to_string(), Value::Decimal(0.1), Some(InputSettings::DragValue { clamp: Some((0.01, 1.0)), speed: Some(0.001) }), None),
            Input::new("bandwidth".to_string(), Value::Decimal(1.5), Some(InputSettings::DragValue { clamp: Some((0.1, 10.0)), speed: Some(0.1) }), None),
            Input::new("density".to_string(), Value::Decimal(16.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.5) }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords, seed, and channel.
    fn hash(ix: i32, iy: i32, impulse: u32, seed: u32, channel: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ impulse.wrapping_mul(2654435761)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Evaluates a single Gabor kernel at a given displacement from the kernel center.
    ///
    /// The kernel is a cosine wave oriented at `angle` radians, modulated by a
    /// Gaussian envelope with standard deviation `sigma`. The `freq` parameter
    /// controls the spatial frequency of the cosine wave.
    fn gabor_kernel(dx: f64, dy: f64, freq: f64, angle: f64, sigma: f64) -> f64 {
        // Rotate displacement into kernel's local coordinate frame
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let rx = dx * cos_a + dy * sin_a;
        let _ry = -dx * sin_a + dy * cos_a;

        // Gaussian envelope
        let gaussian = (-0.5 * (dx * dx + dy * dy) / (sigma * sigma)).exp();

        // Oriented cosine wave
        let wave = (std::f64::consts::TAU * freq * rx).cos();

        gaussian * wave
    }

    /// Generates a Gabor noise image from the given inputs.
    ///
    /// For each cell in a grid determined by `density`, places a number of impulse
    /// kernels with jittered positions. For each pixel, sums contributions from
    /// nearby kernels within a truncation radius (3 * sigma). The kernel's Gaussian
    /// envelope ensures contributions decay to zero outside this radius.
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

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(orientation) = orientation_converted.unwrap() else { unreachable!() };
        let Value::Bool(random_orientation) = random_orient_converted.unwrap() else { unreachable!() };
        let Value::Decimal(kernel_freq) = kernel_freq_converted.unwrap() else { unreachable!() };
        let Value::Decimal(bandwidth) = bandwidth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let orientation_rad = (orientation as f64).to_radians();
        let kernel_freq = kernel_freq as f64;
        let bandwidth = (bandwidth as f64).max(0.1);
        let density = (density as f64).max(1.0);

        // Sigma derived from bandwidth: controls how wide each kernel is
        let sigma = bandwidth / density;
        // Truncation radius: kernels beyond this distance contribute negligibly
        let truncation = 3.0 * sigma;

        let grid_size = density.ceil() as i32;
        // Number of impulses (kernels) per grid cell
        let impulses_per_cell = 1u32;

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // For each pixel, find nearby grid cells and sum kernel contributions (parallelized)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position in grid space
                let gx = (px as f64 / w as f64) * density;
                let gy = (py as f64 / h as f64) * density;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                // Search radius in cells (based on truncation distance)
                let search = (truncation * density / density).ceil() as i32 + 1;

                let mut sum = 0.0;

                for dy in -search..=search {
                    for dx in -search..=search {
                        let mut cx = cell_x + dx;
                        let mut cy = cell_y + dy;

                        cx = cx.rem_euclid(grid_size);
                        cy = cy.rem_euclid(grid_size);

                        for imp in 0..impulses_per_cell {
                            // Kernel position within cell (jittered)
                            let kx = (cell_x + dx) as f64 + Self::hash(cx, cy, imp, seed_u32, 0);
                            let ky = (cell_y + dy) as f64 + Self::hash(cx, cy, imp, seed_u32, 1);

                            let disp_x = (gx - kx) / density;
                            let disp_y = (gy - ky) / density;
                            let dist_sq = disp_x * disp_x + disp_y * disp_y;

                            // Skip kernels outside truncation radius
                            if dist_sq > truncation * truncation {
                                continue;
                            }

                            // Kernel weight (random sign for more interesting patterns)
                            let weight = Self::hash(cx, cy, imp, seed_u32, 2) * 2.0 - 1.0;

                            // Per-kernel orientation
                            let angle = if random_orientation {
                                Self::hash(cx, cy, imp, seed_u32, 3) * std::f64::consts::TAU
                            } else {
                                orientation_rad
                            };

                            sum += weight * Self::gabor_kernel(disp_x, disp_y, kernel_freq * density, angle, sigma);
                        }
                    }
                }

                sum
            })
        }).collect();

        // Normalize to [0, 1]
        let min_val = buffer.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = buffer.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max_val - min_val).max(1e-10);

        // Build a single-channel FloatImage from the normalized buffer values
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let normalized = ((buffer[y * w + x] - min_val) / range) as f32;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(normalized);
                float_image.put_pixel(x as u32, y as u32, &[non_linear]);
            }
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "gabor_tests.rs"]
mod tests;
