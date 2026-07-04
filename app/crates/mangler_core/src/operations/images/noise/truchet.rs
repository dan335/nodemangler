//! Truchet tiles image generator.
//!
//! Produces a grayscale image of Truchet tiles: a square grid where each cell
//! randomly picks one of two orientations of the same motif. Quarter-circle
//! arcs meet at every cell-edge midpoint, so the random tiles connect into
//! continuous maze-like pipes; the diagonal variant produces angular circuitry.
//!
//! Inherently seamless: the random orientation bits wrap at the grid
//! boundaries and every motif meets its neighbors at edge midpoints.

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

/// Operation that generates a Truchet tile pattern image.
///
/// Each grid cell hashes to one bit choosing between two mirrored motifs:
/// two quarter-circle arcs (radius 0.5, centered on opposite corners) or two
/// diagonal segments joining edge midpoints. Lines are drawn as smooth bands
/// of `line_width` around the motif curves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseTruchet {}

impl OpImageNoiseTruchet {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "truchet tiles".to_string(),
            description: "Random quarter-circle or diagonal tiles that connect into maze-like pipes. Creates sci-fi panels, circuitry, and trim patterns.".to_string(),
            help: "Classic Truchet tiling: every grid cell randomly picks one of two orientations of the same motif. Because the motif always meets the cell edges at their midpoints, the random choices join into continuous winding paths.\n\nThe arc motif draws two quarter circles centered on opposite cell corners, producing smooth looping pipes. The diagonal motif joins edge midpoints with straight segments for an angular, circuit-board look. Line width is relative to a cell; softness controls edge antialiasing.\n\nGreat for sci-fi panels, greebles, circuitry, mosaics, and trim sheets - and a strong base for warps and bevels.".to_string(),
        }
    }

    /// Creates the default inputs for the Truchet tiles operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for tile orientations; change to reroute the paths."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Integer(8), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None)
                .with_description("Number of tiles across the image."),
            Input::new("line_width".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.01, 0.5), step_by: None, clamp_to_range: true }), None)
                .with_description("Width of the drawn lines relative to a tile."),
            Input::new("softness".to_string(), Value::Decimal(0.02), Some(InputSettings::Slider { range: (0.0, 0.25), step_by: None, clamp_to_range: true }), None)
                .with_description("Edge softness of the lines; 0 is hard-edged, higher values feather the band."),
            Input::new("diagonal".to_string(), Value::Bool(false), None, None)
                .with_description("When true uses straight diagonal segments instead of quarter-circle arcs."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale Truchet pattern of connected white paths on black."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords and seed.
    #[inline(always)]
    fn hash(ix: i32, iy: i32, seed: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ seed.wrapping_mul(1013904223);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Distance from a point to the arc motif: two quarter circles of radius
    /// 0.5 centered on opposite cell corners. `flipped` picks which corner pair.
    #[inline(always)]
    fn arc_distance(fx: f64, fy: f64, flipped: bool) -> f64 {
        let (c1, c2) = if flipped {
            ((1.0, 0.0), (0.0, 1.0))
        } else {
            ((0.0, 0.0), (1.0, 1.0))
        };
        let d1 = (((fx - c1.0) * (fx - c1.0) + (fy - c1.1) * (fy - c1.1)).sqrt() - 0.5).abs();
        let d2 = (((fx - c2.0) * (fx - c2.0) + (fy - c2.1) * (fy - c2.1)).sqrt() - 0.5).abs();
        d1.min(d2)
    }

    /// Distance from a point to a line segment from `a` to `b`.
    #[inline(always)]
    fn segment_distance(fx: f64, fy: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
        let abx = b.0 - a.0;
        let aby = b.1 - a.1;
        let apx = fx - a.0;
        let apy = fy - a.1;
        let t = ((apx * abx + apy * aby) / (abx * abx + aby * aby)).clamp(0.0, 1.0);
        let dx = apx - t * abx;
        let dy = apy - t * aby;
        (dx * dx + dy * dy).sqrt()
    }

    /// Distance from a point to the diagonal motif: two segments joining edge
    /// midpoints. `flipped` picks which pair of opposite corners they skirt.
    #[inline(always)]
    fn diagonal_distance(fx: f64, fy: f64, flipped: bool) -> f64 {
        let (s1, s2) = if flipped {
            (((0.5, 0.0), (1.0, 0.5)), ((0.0, 0.5), (0.5, 1.0)))
        } else {
            (((0.0, 0.5), (0.5, 0.0)), ((0.5, 1.0), (1.0, 0.5)))
        };
        let d1 = Self::segment_distance(fx, fy, s1.0, s1.1);
        let d2 = Self::segment_distance(fx, fy, s2.0, s2.1);
        d1.min(d2)
    }

    /// Generates a Truchet tile image from the given inputs.
    ///
    /// Each pixel finds its cell, hashes the wrapped cell coordinates to choose
    /// the motif orientation, computes the distance to the motif curves, and
    /// draws a smooth band of `line_width` around them.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let line_width_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let softness_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let diagonal_converted = convert_input(inputs, 6, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(line_width) = line_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(softness) = softness_converted.unwrap() else { unreachable!() };
        let Value::Bool(diagonal) = diagonal_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let grid_size = density.clamp(1, 64);
        let line_width = (line_width as f64).clamp(0.01, 0.5);
        let softness = (softness as f64).clamp(0.0, 0.25);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let half_width = line_width * 0.5;

        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position in grid space (tile units)
                let gx = (px as f64 / w as f64) * grid_size as f64;
                let gy = (py as f64 / h as f64) * grid_size as f64;

                let cell_x = (gx.floor() as i32).rem_euclid(grid_size);
                let cell_y = (gy.floor() as i32).rem_euclid(grid_size);
                let fx = gx.fract();
                let fy = gy.fract();

                // One random bit per cell picks the motif orientation
                let flipped = Self::hash(cell_x, cell_y, seed_u32) < 0.5;

                let d = if diagonal {
                    Self::diagonal_distance(fx, fy, flipped)
                } else {
                    Self::arc_distance(fx, fy, flipped)
                };

                // Smooth band around the motif curve
                if softness <= 0.0 {
                    if d <= half_width { 1.0 } else { 0.0 }
                } else {
                    let t = ((half_width + softness - d) / softness).clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                }
            })
        }).collect();

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
#[path = "truchet_tests.rs"]
mod tests;
