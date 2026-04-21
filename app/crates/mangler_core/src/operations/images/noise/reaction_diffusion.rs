//! Reaction-diffusion (Gray-Scott model) image generator.
//!
//! Simulates two interacting chemicals (activator A and inhibitor B) diffusing
//! across a grid. Depending on the feed and kill rates, produces spots, worms,
//! maze, coral, mitosis, and other organic patterns impossible to achieve with
//! standard noise functions.
//!
//! Always tiles seamlessly by wrapping the grid edges during diffusion.

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

/// Operation that generates organic patterns via reaction-diffusion simulation.
///
/// Uses the Gray-Scott model where two chemicals diffuse and react on a 2D grid.
/// The `feed` rate controls how quickly chemical A is replenished, and the `kill`
/// rate controls how quickly chemical B decays. Different feed/kill combinations
/// produce radically different patterns:
/// - (0.055, 0.062): spots
/// - (0.035, 0.065): worms/stripes
/// - (0.029, 0.057): maze/labyrinth
/// - (0.025, 0.060): coral/branching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseReactionDiffusion {}

impl OpImageNoiseReactionDiffusion {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "reaction diffusion".to_string(),
            description: "Gray-Scott reaction-diffusion simulation producing organic spots, worms, maze, and coral patterns.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, feed/kill rates, and iterations.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(256), Some(InputSettings::DragValue { clamp: Some((1.0, 2048.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(256), Some(InputSettings::DragValue { clamp: Some((1.0, 2048.0)), speed: None }), None),
            Input::new("feed".to_string(), Value::Decimal(0.055), Some(InputSettings::DragValue { clamp: Some((0.0, 0.1)), speed: Some(0.0001) }), None),
            Input::new("kill".to_string(), Value::Decimal(0.062), Some(InputSettings::DragValue { clamp: Some((0.0, 0.1)), speed: Some(0.0001) }), None),
            Input::new("diffusion_a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: Some((0.0, 2.0)), speed: Some(0.01) }), None),
            Input::new("diffusion_b".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: Some((0.0, 2.0)), speed: Some(0.01) }), None),
            Input::new("iterations".to_string(), Value::Integer(4000), Some(InputSettings::DragValue { clamp: Some((100.0, 50000.0)), speed: Some(100.0) }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Simple hash for seeding initial perturbation spots.
    fn hash(x: u32, y: u32, seed: u32) -> u32 {
        let mut h = x.wrapping_mul(1597334677)
            ^ y.wrapping_mul(2943785939)
            ^ seed.wrapping_mul(1013904223);
        h = h.wrapping_mul(h ^ (h >> 16));
        h
    }

    /// Generates a reaction-diffusion pattern image from the given inputs.
    ///
    /// Algorithm (Gray-Scott model):
    /// 1. Initialize grid with A=1, B=0 everywhere
    /// 2. Seed random spots where B=1 based on the seed value
    /// 3. For each iteration, compute the Laplacian of A and B using a 3x3 stencil
    /// 4. Update: A += (Da * laplacian_A - A*B*B + feed*(1-A)) * dt
    /// 5. Update: B += (Db * laplacian_B + A*B*B - (kill+feed)*B) * dt
    /// 6. Output the B channel as grayscale (inverted so patterns are bright)
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let feed_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let kill_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let da_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let db_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let iterations_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(feed) = feed_converted.unwrap() else { unreachable!() };
        let Value::Decimal(kill) = kill_converted.unwrap() else { unreachable!() };
        let Value::Decimal(da) = da_converted.unwrap() else { unreachable!() };
        let Value::Decimal(db) = db_converted.unwrap() else { unreachable!() };
        let Value::Integer(iterations) = iterations_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let iterations = iterations.max(100) as usize;
        let feed = feed as f64;
        let kill = kill as f64;
        let da = da as f64;
        let db = db as f64;
        let w = width as usize;
        let h = height as usize;

        // Initialize grids: A = 1.0 everywhere, B = 0.0 everywhere
        let mut grid_a = vec![1.0f64; w * h];
        let mut grid_b = vec![0.0f64; w * h];

        // Seed initial B spots using deterministic hash
        // Place several small square patches of B=1 at pseudo-random locations
        let num_spots = (w * h / 400).max(3).min(50);
        for i in 0..num_spots {
            let hx = Self::hash(i as u32, 0, seed as u32);
            let hy = Self::hash(i as u32, 1, seed as u32);
            let cx = (hx % width as u32) as usize;
            let cy = (hy % height as u32) as usize;
            let radius = 3;
            for dy in 0..radius * 2 {
                for dx in 0..radius * 2 {
                    let px = (cx + dx).rem_euclid(w);
                    let py = (cy + dy).rem_euclid(h);
                    grid_b[py * w + px] = 1.0;
                }
            }
        }

        // Pre-compute neighbor index lookup tables with wrapping for seamless tiling.
        let xm_table: Vec<usize> = (0..w).map(|x| (x + w - 1) % w).collect();
        let xp_table: Vec<usize> = (0..w).map(|x| (x + 1) % w).collect();
        let ym_table: Vec<usize> = (0..h).map(|y| (y + h - 1) % h).collect();
        let yp_table: Vec<usize> = (0..h).map(|y| (y + 1) % h).collect();

        let mut next_a = vec![0.0f64; w * h];
        let mut next_b = vec![0.0f64; w * h];

        let kill_feed = kill + feed;

        // Run simulation with parallel row processing.
        for _ in 0..iterations {
            next_a.par_chunks_mut(w)
                .zip(next_b.par_chunks_mut(w))
                .enumerate()
                .for_each(|(y, (row_a, row_b))| {
                    let ym = ym_table[y];
                    let yp = yp_table[y];
                    let row_cur = y * w;
                    let row_ym = ym * w;
                    let row_yp = yp * w;

                    for x in 0..w {
                        let idx = row_cur + x;
                        let a = grid_a[idx];
                        let b = grid_b[idx];
                        let xm = xm_table[x];
                        let xp = xp_table[x];

                        // 3x3 Laplacian stencil
                        let lap_a = (grid_a[row_cur + xm] + grid_a[row_cur + xp]
                            + grid_a[row_ym + x] + grid_a[row_yp + x]) * 0.2
                            + (grid_a[row_ym + xm] + grid_a[row_ym + xp]
                            + grid_a[row_yp + xm] + grid_a[row_yp + xp]) * 0.05
                            - a;

                        let lap_b = (grid_b[row_cur + xm] + grid_b[row_cur + xp]
                            + grid_b[row_ym + x] + grid_b[row_yp + x]) * 0.2
                            + (grid_b[row_ym + xm] + grid_b[row_ym + xp]
                            + grid_b[row_yp + xm] + grid_b[row_yp + xp]) * 0.05
                            - b;

                        let abb = a * b * b;
                        // Gray-Scott update with implicit dt=1.0 baked into the equations.
                        row_a[x] = (a + da * lap_a - abb + feed * (1.0 - a)).clamp(0.0, 1.0);
                        row_b[x] = (b + db * lap_b + abb - kill_feed * b).clamp(0.0, 1.0);
                    }
                });

            std::mem::swap(&mut grid_a, &mut next_a);
            std::mem::swap(&mut grid_b, &mut next_b);
        }

        // Output the B channel (inverted: patterns appear bright on dark background)
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let b = grid_b[y * w + x] as f32;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(b);
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
#[path = "reaction_diffusion_tests.rs"]
mod tests;
