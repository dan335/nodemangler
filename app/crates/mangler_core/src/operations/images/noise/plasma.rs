//! Plasma noise image generator.
//!
//! Produces a seamlessly tiling grayscale image using the diamond-square algorithm,
//! a classic fractal subdivision technique that produces organic, plasma-like patterns.
//! The grid wraps at boundaries for seamless tiling: edge and corner values are shared
//! between opposite sides of the image.

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

/// Simple seeded pseudo-random number generator for reproducible plasma patterns.
///
/// Uses an xorshift32 algorithm for fast, stateful random number generation
/// during the diamond-square subdivision process.
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    /// Creates a new RNG with the given seed. Ensures state is never zero.
    fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    /// Returns the next pseudo-random f64 in [-1, 1] using xorshift32.
    fn next_f64(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f64 / u32::MAX as f64) * 2.0 - 1.0
    }
}

/// Runs the diamond-square algorithm on a `(size+1) x (size+1)` grid with
/// periodic wrapping for seamless tiling.
///
/// `size` must be a power of 2. The grid wraps so that `grid[size][y] == grid[0][y]`
/// and `grid[x][size] == grid[x][0]`, producing seamless edges.
/// `roughness` controls how quickly random displacement decays with subdivision depth.
fn diamond_square_tiling(size: usize, seed: u32, roughness: f64) -> Vec<f64> {
    let n = size + 1;
    let mut grid = vec![0.0f64; n * n];
    let mut rng = SimpleRng::new(seed);

    // Wrap helper: reads from the grid with periodic boundary
    let idx = |x: usize, y: usize| -> usize {
        (y % size) * n + (x % size)
    };

    // Seed corners (all map to the same wrapped point for tiling)
    grid[idx(0, 0)] = rng.next_f64();

    let mut step = size;
    let mut scale = roughness;

    while step > 1 {
        let half = step / 2;

        // Diamond step: set center of each square to average of corners + random offset
        let mut y = half;
        while y < size {
            let mut x = half;
            while x < size {
                let tl = grid[idx(x.wrapping_sub(half), y.wrapping_sub(half))];
                let tr = grid[idx((x + half) % size, y.wrapping_sub(half))];
                let bl = grid[idx(x.wrapping_sub(half), (y + half) % size)];
                let br = grid[idx((x + half) % size, (y + half) % size)];
                let avg = (tl + tr + bl + br) / 4.0;
                grid[idx(x, y)] = avg + rng.next_f64() * scale;
                x += step;
            }
            y += step;
        }

        // Square step: set edge midpoints to average of diamond corners + random offset
        let mut y = 0;
        while y < size {
            let mut x = 0;
            while x < size {
                // Right edge midpoint
                if half > 0 {
                    let mid_x = (x + half) % size;
                    let mid_y = y;
                    let left = grid[idx(x, mid_y)];
                    let right = grid[idx((x + step) % size, mid_y)];
                    let top = grid[idx(mid_x, (mid_y + size - half) % size)];
                    let bottom = grid[idx(mid_x, (mid_y + half) % size)];
                    let avg = (left + right + top + bottom) / 4.0;
                    grid[idx(mid_x, mid_y)] = avg + rng.next_f64() * scale;
                }

                // Bottom edge midpoint
                if half > 0 {
                    let mid_x = x;
                    let mid_y = (y + half) % size;
                    let top = grid[idx(mid_x, y)];
                    let bottom = grid[idx(mid_x, (y + step) % size)];
                    let left = grid[idx((mid_x + size - half) % size, mid_y)];
                    let right = grid[idx((mid_x + half) % size, mid_y)];
                    let avg = (top + bottom + left + right) / 4.0;
                    grid[idx(mid_x, mid_y)] = avg + rng.next_f64() * scale;
                }
                x += step;
            }
            y += step;
        }

        step = half;
        scale *= roughness;
    }

    // Normalize to [0, 1]
    let min = grid.iter().copied().fold(f64::INFINITY, f64::min);
    let max = grid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range > 0.0 {
        for v in grid.iter_mut() {
            *v = (*v - min) / range;
        }
    }

    grid
}

/// Operation that generates a seamlessly tiling plasma noise image.
///
/// Uses the diamond-square algorithm with periodic wrapping for seamless tiling.
/// The `roughness` parameter controls fractal detail: lower values produce
/// smoother gradients, higher values produce more jagged plasma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoisePlasma {}

impl OpImageNoisePlasma {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "plasma noise".to_string(),
            description: "Creates a seamlessly tiling plasma fractal image using diamond-square subdivision.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, detail (grid subdivision level), and roughness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("detail".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (2.0, 12.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("roughness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates a plasma noise image from the given inputs.
    ///
    /// The diamond-square algorithm runs on a power-of-2 grid, then the result
    /// is bilinearly resampled to the requested output dimensions.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let detail_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let roughness_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(detail) = detail_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roughness) = roughness_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let grid_power = detail.clamp(2, 12) as u32;
        let grid_size = 1usize << grid_power; // Power-of-2 grid size
        let roughness = (roughness as f64).clamp(0.01, 1.0);

        // Run diamond-square on the power-of-2 grid
        let grid = diamond_square_tiling(grid_size, seed as u32, roughness);
        let gn = grid_size + 1;

        let w = width as usize;
        let h = height as usize;

        // Bilinearly resample the grid to the output dimensions (parallelized with rayon)
        let grid_ref = &grid;
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
            (0..w).map(move |x| {
                // Map output pixel to grid coordinates [0, grid_size), wrapping for tiling
                let gx = x as f64 / w as f64 * grid_size as f64;
                let gy = y as f64 / h as f64 * grid_size as f64;

                let x0 = gx.floor() as usize % grid_size;
                let y0 = gy.floor() as usize % grid_size;
                let x1 = (x0 + 1) % grid_size;
                let y1 = (y0 + 1) % grid_size;

                let fx = gx.fract();
                let fy = gy.fract();

                // Bilinear interpolation
                let v00 = grid_ref[y0 * gn + x0];
                let v10 = grid_ref[y0 * gn + x1];
                let v01 = grid_ref[y1 * gn + x0];
                let v11 = grid_ref[y1 * gn + x1];

                let top = v00 + (v10 - v00) * fx;
                let bot = v01 + (v11 - v01) * fx;
                let noise = (top + (bot - top) * fy) as f32;

                linear_to_nonlinear_srgb(noise.clamp(0.0, 1.0))
            })
        }).collect();

        // Build a single-channel FloatImage from the computed pixel values
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for (i, &val) in pixels.iter().enumerate() {
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            float_image.put_pixel(x, y, &[val]);
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
#[path = "plasma_tests.rs"]
mod tests;
