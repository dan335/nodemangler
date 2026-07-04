//! Worley (cellular) noise distance image generator.
//!
//! Produces a grayscale image based on the distance to the nearest cell point
//! in a Worley noise field. Uses a custom grid-based implementation with
//! seamless tiling via `rem_euclid` wrapping. Supports multiple distance
//! functions: Chebyshev, Euclidean, Euclidean squared, Manhattan, and Quadratic.
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

pub use crate::operations::images::noise::voronoi_common::NoiseWorleyDistanceFunction;
use crate::operations::images::noise::voronoi_common::{cell_hash, compute_distance, grid_size_from_frequency, pixel_to_grid, wrap_cell};

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
            name: "worley distance noise".to_string(),
            description: "Creates a seamlessly tiling worley noise distance image.".to_string(),
            help: "Worley (cellular) noise: jittered points are scattered on a grid and each pixel outputs the distance to its nearest point (F1). Cell interiors are dark near the seed point and brighten outward, so the result reads as bumpy cells with smooth gradients between seeds and boundaries.\n\nFrequency controls how many cells span the tile; higher values shrink each cell. The distance function (Euclidean, Manhattan, Chebyshev, Euclidean-squared, or Quadratic) changes cell shape: Euclidean rounds, Manhattan gives diamond facets, Chebyshev gives square cells.\n\nUseful for stones, pebbles, organic bumpy surfaces, water caustics, and cellular height/normal maps.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, distance function, and frequency.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed placing the Worley cell points."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None)
                .with_description("Distance metric used to measure nearest-point distance; shapes the cell appearance."),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::Slider { range: (1.0, 50.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Number of cells across the tile; higher values produce smaller cells."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale Worley noise where brightness reflects distance to nearest cell point."),
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

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
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
#[path = "worley_distance_tests.rs"]
mod tests;
