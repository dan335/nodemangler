//! Voronoise image generator (Inigo Quilez technique).
//!
//! Produces a grayscale image that smoothly blends between gradient noise and
//! Voronoi/cellular noise using two parameters: `jitter` controls the regularity
//! of the cell grid (0 = regular grid, 1 = fully random), and `smoothness` controls
//! interpolation between sharp Voronoi cells and smooth gradient noise.
//!
//! At (jitter=0, smoothness=0) it produces a regular grid pattern.
//! At (jitter=1, smoothness=0) it produces sharp Voronoi cells.
//! At (jitter=0, smoothness=1) it produces smooth value noise.
//! At (jitter=1, smoothness=1) it produces smooth organic noise.
//!
//! Based on Inigo Quilez's "voronoise" function.
//! Always tiles seamlessly — frequency is rounded to the nearest integer so the
//! grid wraps cleanly at image boundaries.

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

use super::voronoi_common::{cell_hash, grid_size_from_frequency, pixel_to_grid, wrap_cell};

/// Operation that generates a voronoise image blending Voronoi and gradient noise.
///
/// The `jitter` parameter controls cell point randomness and the `smoothness`
/// parameter controls interpolation sharpness, allowing continuous blending
/// between regular grids, Voronoi cells, and smooth gradient noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseVoronoise {}

impl OpImageNoiseVoronoise {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "voronoi blend".to_string(),
            description: "Blends between Voronoi and gradient noise. Jitter controls cell randomness, smoothness controls interpolation.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, frequency, jitter, and smoothness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("frequency".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((1.0, 100.0)), speed: Some(0.1) }), None),
            Input::new("jitter".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("smoothness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Evaluates the voronoise function at the given position.
    ///
    /// Implementation based on Inigo Quilez's voronoise:
    /// - Iterates over a 5x5 cell neighborhood
    /// - Computes weighted contributions from each cell's random point
    /// - Weight is based on distance raised to a power controlled by smoothness
    /// - Jitter controls how much cell points deviate from cell centers
    fn eval(px: f64, py: f64, jitter: f64, smoothness: f64, seed: u32, grid_size: i32) -> f64 {
        let cell_x = px.floor() as i32;
        let cell_y = py.floor() as i32;
        let frac_x = px - cell_x as f64;
        let frac_y = py - cell_y as f64;

        // Smoothness controls the sharpness of the kernel: higher = smoother blending
        // Map from [0,1] to a useful exponent range
        let k = 1.0 + 63.0 * (1.0 - smoothness).powi(6);

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        for dy in -2..=2 {
            for dx in -2..=2 {
                let nx = wrap_cell(cell_x + dx, grid_size);
                let ny = wrap_cell(cell_y + dy, grid_size);

                let hx = cell_hash(nx, ny, seed, 0);
                let hy = cell_hash(nx, ny, seed, 1);
                let hval = cell_hash(nx, ny, seed, 2);

                // Jittered point position relative to current fragment
                let ox = dx as f64 + hx * jitter - frac_x;
                let oy = dy as f64 + hy * jitter - frac_y;

                // Squared distance to cell point
                let d = ox * ox + oy * oy;

                // Weight falls off with distance, sharpness controlled by k
                let w = (-k * d).exp();

                weighted_sum += hval * w;
                weight_total += w;
            }
        }

        if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.5
        }
    }

    /// Generates a voronoise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let jitter_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let smoothness_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(jitter) = jitter_converted.unwrap() else { unreachable!() };
        let Value::Decimal(smoothness) = smoothness_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let jitter = (jitter as f64).clamp(0.0, 1.0);
        let smoothness = (smoothness as f64).clamp(0.0, 1.0);
        let grid_size = grid_size_from_frequency(frequency as f64);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let px = pixel_to_grid(x, w, grid_size);
                let py = pixel_to_grid(y, h, grid_size);

                let noise = Self::eval(px, py, jitter, smoothness, seed_u32, grid_size) as f32;
                let noise = noise.clamp(0.0, 1.0);
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
#[path = "voronoise_tests.rs"]
mod tests;
