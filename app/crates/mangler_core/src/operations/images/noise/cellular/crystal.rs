//! Crystal noise image generator.
//!
//! Produces a seamlessly tiling grayscale image with crystalline/mineral-like
//! patterns. Uses Voronoi cell generation (reusing `voronoi_common` helpers) with
//! each cell assigned a random flat brightness, creating sharp faceted edges
//! between cells — like looking at a cut gemstone or mineral cross-section.

use rayon::prelude::*;
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_output};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::operations::images::noise::voronoi_common::{cell_hash, compute_distance, grid_size_from_frequency, pixel_to_grid, wrap_cell, NoiseWorleyDistanceFunction};

/// Operation that generates a seamlessly tiling crystal noise image.
///
/// Each Voronoi cell is filled with a flat random brightness, producing
/// sharp-edged faceted patterns. The distance function controls cell shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCrystal {}

impl OpImageNoiseCrystal {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "crystal noise".to_string(),
            description: "Creates a seamlessly tiling image with crystalline faceted patterns.".to_string(),
            help: "A Voronoi/cellular-style generator: jittered points are scattered on a grid and each pixel is filled with the flat brightness of its nearest point's cell. The result is a mosaic of sharp-edged facets with no internal gradient.\n\nFrequency controls how many cells fit across the tile, so higher values shrink the facets. The distance function (Euclidean, Manhattan, Chebyshev, etc.) changes how cells meet: Euclidean gives natural angles, Manhattan and Chebyshev give rectilinear facets.\n\nGood for gemstones, mineral cross-sections, stained glass, and low-poly facet looks.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, distance function, and frequency.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed placing the crystal cell points and their brightness values."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None)
                .with_description("Distance metric used to assign each pixel to a cell; shapes the facet edges."),
            Input::new("frequency".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 50.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Number of crystal cells across the image; higher values make smaller facets."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Seamlessly tiling grayscale image of flat-shaded crystalline facets."),
        ]
    }

    /// Generates a crystal noise image from the given inputs.
    ///
    /// For each pixel, finds the nearest Voronoi cell point and outputs that
    /// cell's random brightness value. This gives each cell a flat color with
    /// sharp edges between cells.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(mut seed) = 0,
            Integer(mut width) = 1,
            Integer(mut height) = 2,
            NoiseWorleyDistanceFunction(distance_function) = 3,
            Decimal(frequency) = 4,
        }

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let grid_size = grid_size_from_frequency(frequency as f64);
        let seed_u32 = seed as u32;

        let w = width as usize;
        let h = height as usize;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let px = pixel_to_grid(x, w, grid_size);
                let py = pixel_to_grid(y, h, grid_size);

                let cell_x = px.floor() as i32;
                let cell_y = py.floor() as i32;

                // Find nearest cell point in 3x3 neighborhood
                let mut min_dist = f64::MAX;
                let mut nearest_cx: i32 = 0;
                let mut nearest_cy: i32 = 0;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = wrap_cell(cell_x + dx, grid_size);
                        let ny = wrap_cell(cell_y + dy, grid_size);

                        let point_x = (cell_x + dx) as f64 + cell_hash(nx, ny, seed_u32, 0);
                        let point_y = (cell_y + dy) as f64 + cell_hash(nx, ny, seed_u32, 1);

                        let dist = compute_distance(px, py, point_x, point_y, distance_function);
                        if dist < min_dist {
                            min_dist = dist;
                            nearest_cx = nx;
                            nearest_cy = ny;
                        }
                    }
                }

                // Use the nearest cell's hash as its flat brightness (channel 2 for independence)
                let noise = cell_hash(nearest_cx, nearest_cy, seed_u32, 2) as f32;
                linear_to_nonlinear_srgb(noise)
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
#[path = "crystal_tests.rs"]
mod tests;
