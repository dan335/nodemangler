//! Erosion noise image generator.
//!
//! Generates a base fBm noise heightmap and then applies thermal erosion to
//! create weathered, worn surface textures. Material is transferred from steep
//! slopes to lower neighbors, simulating natural rock and soil weathering.
//!
//! Always tiles seamlessly by wrapping the grid edges during erosion.

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

/// Operation that generates an eroded noise heightmap.
///
/// First generates an fBm noise heightmap, then applies iterative thermal erosion
/// where material flows from steep slopes to lower neighbors. The `talus` angle
/// controls the maximum stable slope, and `erosion_amount` controls how much
/// material moves per iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseErosion {}

impl OpImageNoiseErosion {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "erosion".to_string(),
            description: "Applies thermal erosion to fractal noise, creating weathered stone and terrain textures.".to_string(),
            help: "Starts from an fBm heightmap and then runs a thermal-erosion simulation: on each iteration, every cell compares itself to its eight neighbors and transfers material down the steepest slope whose height difference exceeds the talus angle.\n\nFrequency and octaves shape the initial terrain. Talus sets the maximum stable slope, so smaller values produce smoother, more weathered surfaces. Erosion amount is the fraction of excess material moved per pass; iterations is how many passes run.\n\nProduces weathered rock, eroded mountains, dunes, and soft terrain heightmaps.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, noise params, and erosion params.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the starting heightmap before erosion."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the base heightmap before erosion runs."),
            Input::new("frequency".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Base frequency of the initial heightmap; higher values make smaller terrain features."),
            Input::new("talus".to_string(), Value::Decimal(0.03), Some(InputSettings::DragValue { clamp: Some((0.001, 0.5)), speed: Some(0.001) }), None)
                .with_description("Maximum stable slope; material transfers only when a neighbor exceeds this height difference."),
            Input::new("erosion_amount".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Fraction of excess slope material moved per iteration; higher values erode faster."),
            Input::new("iterations".to_string(), Value::Integer(50), Some(InputSettings::DragValue { clamp: Some((1.0, 500.0)), speed: Some(1.0) }), None)
                .with_description("Number of thermal-erosion passes; more iterations smooth the terrain further."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale heightmap after thermal erosion weathering."),
        ]
    }

    /// Generates an eroded noise image from the given inputs.
    ///
    /// Algorithm:
    /// 1. Generate a base heightmap using torus-mapped fBm noise for seamless tiling
    /// 2. For each erosion iteration, scan all cells
    /// 3. For each cell, find the lowest neighbor (with wrapping edges)
    /// 4. If the height difference exceeds the talus angle threshold, transfer
    ///    material proportional to erosion_amount from the high cell to the low cell
    /// 5. Normalize the result to [0, 1] and output
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let octaves_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let talus_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let erosion_amount_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let iterations_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(talus) = talus_converted.unwrap() else { unreachable!() };
        let Value::Decimal(erosion_amount) = erosion_amount_converted.unwrap() else { unreachable!() };
        let Value::Integer(iterations) = iterations_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let iterations = iterations.max(1) as usize;
        let talus = talus as f64;
        let erosion_amount = (erosion_amount as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;

        // Generate base heightmap from fBm noise
        let fbm = Fbm::<Perlin>::new(seed as u32)
            .set_frequency(frequency as f64)
            .set_octaves(octaves as usize)
            .set_lacunarity(2.094_395_2)
            .set_persistence(0.5);

        let fbm_ref = &fbm;

        // Generate base heightmap from torus-mapped fBm noise for seamless tiling (parallelized)
        let mut heightmap: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let tau = std::f64::consts::TAU;
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let r = 1.0 / tau;
                let noise = fbm_ref.get([
                    (tau * u).cos() * r,
                    (tau * u).sin() * r,
                    (tau * v).cos() * r,
                    (tau * v).sin() * r,
                ]);
                noise * 0.5 + 0.5
            })
        }).collect();

        // Pre-compute neighbor index lookup tables with wrapping for seamless tiling.
        let xm_table: Vec<usize> = (0..w).map(|x| (x + w - 1) % w).collect();
        let xp_table: Vec<usize> = (0..w).map(|x| (x + 1) % w).collect();
        let ym_table: Vec<usize> = (0..h).map(|y| (y + h - 1) % h).collect();
        let yp_table: Vec<usize> = (0..h).map(|y| (y + 1) % h).collect();

        // Reused snapshot buffer: read from the previous state while writing
        // to the live grid. This prevents order-dependent artifacts within a
        // single iteration. Note the transfers write to arbitrary neighbors
        // of the live grid, so the scan itself is sequential by design.
        let mut snapshot = vec![0.0f64; w * h];

        // Apply thermal erosion
        for _ in 0..iterations {
            snapshot.copy_from_slice(&heightmap);

            for y in 0..h {
                let row_ym = ym_table[y] * w;
                let row_y = y * w;
                let row_yp = yp_table[y] * w;

                for x in 0..w {
                    let idx = row_y + x;
                    let current_h = snapshot[idx];
                    let xm = xm_table[x];
                    let xp = xp_table[x];

                    // 8-connected neighbor indices, same order as the original
                    // offset list so tie-breaking is unchanged.
                    let neighbor_indices = [
                        row_ym + xm, row_ym + x, row_ym + xp,
                        row_y + xm,               row_y + xp,
                        row_yp + xm, row_yp + x, row_yp + xp,
                    ];

                    // Find the neighbor with the maximum height difference below current cell
                    let mut max_diff = 0.0f64;
                    let mut max_idx = idx;

                    for &neighbor_idx in &neighbor_indices {
                        let diff = current_h - snapshot[neighbor_idx];
                        if diff > max_diff {
                            max_diff = diff;
                            max_idx = neighbor_idx;
                        }
                    }

                    // Transfer material if slope exceeds talus angle
                    if max_diff > talus && max_idx != idx {
                        let transfer = (max_diff - talus) * 0.5 * erosion_amount;
                        heightmap[idx] -= transfer;
                        heightmap[max_idx] += transfer;
                    }
                }
            }
        }

        // Normalize to [0, 1]
        let min_h = heightmap.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_h = heightmap.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max_h - min_h).max(1e-10);

        // Convert heightmap to image pixels (parallelized)
        let pixels: Vec<f32> = heightmap.par_iter().map(|&val| {
            let normalized = ((val - min_h) / range) as f32;
            crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(normalized)
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
#[path = "erosion_tests.rs"]
mod tests;
