//! Worley (cellular) noise distance image generator.
//!
//! Produces a grayscale image based on the distance to the nearest cell point
//! in a Worley noise field. Uses a custom grid-based implementation with
//! seamless tiling via `rem_euclid` wrapping. Supports multiple distance
//! functions: Chebyshev, Euclidean, Euclidean squared, Manhattan, and Quadratic.
//!
//! Always tiles seamlessly — frequency is rounded to the nearest integer so the
//! grid wraps cleanly at image boundaries.

use image::{ImageBuffer, DynamicImage};
use rayon::prelude::*;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

pub use super::voronoi_common::NoiseWorleyDistanceFunction;
use super::voronoi_common::{cell_hash, compute_distance, grid_size_from_frequency, pixel_to_grid, wrap_cell};

/// Operation that generates a Worley noise image using distance return type.
///
/// The output brightness represents the distance from each pixel to the nearest
/// Worley cell point, producing a cellular/Voronoi-like pattern. Always tiles
/// seamlessly using grid-based wrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseWorleyDistance {}

impl OpImageNoiseWorleyDistance {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "worley distance".to_string(),
            description: "Creates a seamlessly tiling worley noise distance image.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, distance function, and frequency.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::Slider { range: (1.0, 50.0), step_by: Some(0.1), clamp_to_range: false }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates a Worley distance noise image from the given inputs.
    ///
    /// For each pixel, finds the nearest cell point in a 3x3 neighborhood and
    /// outputs the distance to it. Cell coordinates wrap via `rem_euclid` for
    /// seamless tiling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let distance_function_converted = convert_input(inputs, 3, ValueType::NoiseWorleyDistanceFunction, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::NoiseWorleyDistanceFunction(distance_function) = distance_function_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let grid_size = grid_size_from_frequency(frequency as f64);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        let pixels: Vec<u16> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let px = pixel_to_grid(x, w, grid_size);
                let py = pixel_to_grid(y, h, grid_size);

                let cell_x = px.floor() as i32;
                let cell_y = py.floor() as i32;

                let mut min_dist = f64::MAX;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = wrap_cell(cell_x + dx, grid_size);
                        let ny = wrap_cell(cell_y + dy, grid_size);

                        let point_x = (cell_x + dx) as f64 + cell_hash(nx, ny, seed_u32, 0);
                        let point_y = (cell_y + dy) as f64 + cell_hash(nx, ny, seed_u32, 1);

                        let dist = compute_distance(px, py, point_x, point_y, distance_function);
                        min_dist = min_dist.min(dist);
                    }
                }

                let noise = (min_dist as f32 * 2.0).clamp(0.0, 1.0);
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(noise);
                (non_linear * 65535.0) as u16
            })
        }).collect();

        let image_buffer = ImageBuffer::from_raw(width as u32, height as u32, pixels).unwrap();
        let dynamic_image = DynamicImage::ImageLuma16(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "worley_distance_tests.rs"]
mod tests;
