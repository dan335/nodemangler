//! Voronoi F2-F1 (cracked) noise image generator.
//!
//! Produces a grayscale image showing crack patterns by computing the difference
//! between the distance to the second-nearest and nearest Voronoi cell points.
//! This highlights cell boundaries as bright lines on a dark background, creating
//! patterns that resemble cracked earth, dried mud, bark, or stone.
//!
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

/// Operation that generates a Voronoi F2-F1 crack pattern image.
///
/// For each pixel, finds the distances to the two nearest Voronoi cell points
/// and outputs their difference (F2 - F1). Cell boundaries appear as bright
/// lines where F1 and F2 are nearly equal, creating a crack/vein pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseVoronoiCrack {}

impl OpImageNoiseVoronoiCrack {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "voronoi crack noise".to_string(),
            description: "Voronoi F2-F1 noise producing crack/vein patterns from cell boundary distances.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, frequency, and jitter.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("frequency".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((1.0, 100.0)), speed: Some(0.1) }), None),
            Input::new("jitter".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates a Voronoi F2-F1 crack noise image from the given inputs.
    ///
    /// For each pixel:
    /// 1. Determines the cell coordinates based on frequency
    /// 2. Searches the 3x3 neighborhood of cells for the two nearest points
    /// 3. Computes F2 - F1 (difference between 2nd and 1st nearest distances)
    /// 4. Normalizes and gamma-corrects the result
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let jitter_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(jitter) = jitter_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let jitter = (jitter as f64).clamp(0.0, 1.0);
        let grid_size = grid_size_from_frequency(frequency as f64);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let px = pixel_to_grid(x, w, grid_size);
                let py = pixel_to_grid(y, h, grid_size);

                let cell_x = px.floor() as i32;
                let cell_y = py.floor() as i32;

                let mut f1 = f64::MAX;
                let mut f2 = f64::MAX;

                // Search 3x3 neighborhood of cells
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = wrap_cell(cell_x + dx, grid_size);
                        let ny = wrap_cell(cell_y + dy, grid_size);

                        // Jittered point position within the cell
                        let point_x = (cell_x + dx) as f64 + 0.5 + (cell_hash(nx, ny, seed_u32, 0) - 0.5) * jitter;
                        let point_y = (cell_y + dy) as f64 + 0.5 + (cell_hash(nx, ny, seed_u32, 1) - 0.5) * jitter;

                        let dist_x = px - point_x;
                        let dist_y = py - point_y;
                        let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();

                        // Track the two nearest distances
                        if dist < f1 {
                            f2 = f1;
                            f1 = dist;
                        } else if dist < f2 {
                            f2 = dist;
                        }
                    }
                }

                // F2 - F1: bright at cell boundaries, dark in cell interiors
                let noise = ((f2 - f1) as f32).clamp(0.0, 1.0);
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

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "voronoi_crack_tests.rs"]
mod tests;
