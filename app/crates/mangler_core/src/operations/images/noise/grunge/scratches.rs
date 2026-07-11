//! Scratches noise image generator.
//!
//! Produces a grayscale image of sparse thin line segments with randomized
//! length, angle, curvature, and intensity. The staple generator for roughness
//! and wear maps: worn metal, scuffed plastic, brushed and scraped surfaces.
//!
//! Each jittered grid cell drops one scratch: a tapered, optionally curved
//! stroke. Scratches use MAX blending so crossings stay crisp. Always tiles
//! seamlessly by wrapping cell coordinates at grid boundaries.

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

/// Operation that generates a scratches noise image.
///
/// Places one curved, tapered line segment per jittered grid cell. Each scratch
/// has randomized center, angle (blended between a shared direction and fully
/// random by `angle_variation`), length, bend, and intensity. Pixels take the
/// MAX contribution of nearby scratches so overlaps stay distinct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseScratches {}

impl OpImageNoiseScratches {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "scratches".to_string(),
            description: "Sparse thin line segments with random length, angle, and curvature. The staple for roughness maps: worn metal, scuffed plastic, scraped surfaces.".to_string(),
            help: "Each jittered grid cell drops one scratch: a thin stroke that tapers to points at both ends and can bend into a shallow arc. Angle variation blends between all scratches sharing the angle input (0) and fully random directions (1). Length and intensity variation randomize per-scratch values. Overlapping scratches use MAX blending so crossings stay crisp instead of blooming.\n\nDensity sets how many scratch cells fit across the tile; length and thickness are relative to a cell.\n\nBest used as a roughness or wear mask: worn metal, scuffed plastic, scraped paint, and brushed surfaces with angle variation near 0.".to_string(),
        }
    }

    /// Creates the default inputs for the scratches operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for scratch placement and shape; change to rearrange the scratches."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.1) }), None)
                .with_description("Number of scratch cells across the image; higher values pack more scratches."),
            Input::new("length".to_string(), Value::Decimal(1.5), Some(InputSettings::DragValue { clamp: Some((0.1, 4.0)), speed: Some(0.01) }), None)
                .with_description("Base scratch length relative to cell size; larger values make longer strokes."),
            Input::new("length_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much scratch lengths vary from the base length; 0 is uniform, 1 is most varied."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Shared scratch direction in degrees when angle variation is low."),
            Input::new("angle_variation".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How far scratch angles stray from the shared angle; 0 aligns all, 1 is fully random."),
            Input::new("curvature".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Maximum bend of each scratch; 0 keeps them straight, 1 allows strong arcs."),
            Input::new("thickness".to_string(), Value::Decimal(0.04), Some(InputSettings::DragValue { clamp: Some((0.002, 0.5)), speed: Some(0.001) }), None)
                .with_description("Stroke thickness relative to cell size; keep small for hairline scratches."),
            Input::new("intensity".to_string(), Value::Decimal(0.8), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Peak brightness of each scratch."),
            Input::new("intensity_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much scratch brightness varies; 0 is uniform, 1 is most varied."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of sparse thin scratches on black."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords, seed, and channel.
    #[inline(always)]
    fn hash(ix: i32, iy: i32, seed: u32, channel: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Evaluates one scratch stroke at a displacement from its center, in cell units.
    ///
    /// The stroke runs along the (`cos_a`, `sin_a`) axis for `half_len` on each
    /// side, bends into a parabolic arc whose apex offset is `bend`, and tapers
    /// to a point at both ends. Returns the stroke coverage in [0, 1].
    #[inline(always)]
    fn scratch_kernel(
        dx: f64,
        dy: f64,
        half_len: f64,
        cos_a: f64,
        sin_a: f64,
        bend: f64,
        thickness: f64,
    ) -> f64 {
        // Rotate displacement into the scratch's local frame:
        // lx runs along the stroke, ly across it.
        let lx = dx * cos_a + dy * sin_a;
        let ly = -dx * sin_a + dy * cos_a;

        let t = lx / half_len;
        if t.abs() >= 1.0 {
            return 0.0;
        }

        // Parabolic arc: apex offset `bend` at the center, zero at both ends.
        let arc = bend * (1.0 - t * t);
        let d = (ly - arc).abs();

        // Taper the stroke width to a point at both ends.
        let taper = (1.0 - t * t).sqrt();
        let local_thickness = thickness * taper;
        if local_thickness <= 0.0 || d >= local_thickness {
            return 0.0;
        }

        // Quadratic cross-profile: opaque core with soft edges.
        let n = d / local_thickness;
        1.0 - n * n
    }

    /// Generates a scratches noise image from the given inputs.
    ///
    /// Divides UV space into a `density` x `density` grid; each cell drops one
    /// scratch at a jittered position with randomized angle, length, bend, and
    /// intensity. Pixels take the MAX contribution of scratches from nearby
    /// cells; cell coordinates wrap for seamless tiling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let length_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let length_var_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let angle_var_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let curvature_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let thickness_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let intensity_var_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(length) = length_converted.unwrap() else { unreachable!() };
        let Value::Decimal(length_variation) = length_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle_variation) = angle_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(curvature) = curvature_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity_variation) = intensity_var_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        // Snap density to an integer so the cell grid and the pixel->grid
        // mapping span the same number of cells; a fractional density leaves a
        // partial final cell at the tile edge and breaks seamless tiling
        // (mirrors voronoi_common::grid_size_from_frequency). Integer densities
        // are unchanged.
        let density = (density as f64).max(1.0).round().max(1.0);
        let length = (length as f64).clamp(0.1, 4.0);
        let length_variation = (length_variation as f64).clamp(0.0, 1.0);
        let angle_rad = (angle as f64).to_radians();
        let angle_variation = (angle_variation as f64).clamp(0.0, 1.0);
        let curvature = (curvature as f64).clamp(0.0, 1.0);
        let thickness = (thickness as f64).clamp(0.002, 0.5);
        let intensity = (intensity as f64).clamp(0.0, 1.0);
        let intensity_variation = (intensity_variation as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let grid_size = density.ceil() as i32;

        // Longest possible scratch half-length in cell units, plus bend and
        // stroke thickness, bounds how far a scratch can reach from its cell.
        let max_half_len = length * (1.0 + length_variation) * 0.5;
        let reach = max_half_len * (1.0 + curvature) + thickness;
        let search = reach.ceil() as i32 + 1;

        // MAX-blend scratch contributions (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position in grid space (cell units)
                let gx = (px as f64 / w as f64) * density;
                let gy = (py as f64 / h as f64) * density;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                let mut max_val = 0.0_f64;

                for dy in -search..=search {
                    for dx in -search..=search {
                        // Wrap cell coordinates for seamless tiling
                        let cx = (cell_x + dx).rem_euclid(grid_size);
                        let cy = (cell_y + dy).rem_euclid(grid_size);

                        // Scratch center jittered within its cell
                        let kx = (cell_x + dx) as f64 + Self::hash(cx, cy, seed_u32, 0);
                        let ky = (cell_y + dy) as f64 + Self::hash(cx, cy, seed_u32, 1);

                        let disp_x = gx - kx;
                        let disp_y = gy - ky;
                        let dist_sq = disp_x * disp_x + disp_y * disp_y;
                        if dist_sq > reach * reach {
                            continue;
                        }

                        // Per-scratch randomized parameters
                        let angle_rand = Self::hash(cx, cy, seed_u32, 2) * 2.0 - 1.0;
                        let scratch_angle = angle_rad + angle_rand * angle_variation * std::f64::consts::PI;

                        let len_rand = Self::hash(cx, cy, seed_u32, 3);
                        let half_len = length * (1.0 - length_variation + length_variation * len_rand * 2.0) * 0.5;

                        let bend_rand = Self::hash(cx, cy, seed_u32, 4) * 2.0 - 1.0;
                        let bend = bend_rand * curvature * half_len;

                        let int_rand = Self::hash(cx, cy, seed_u32, 5);
                        let scratch_intensity = intensity * (1.0 - intensity_variation + intensity_variation * int_rand * 2.0);

                        let kernel_val = Self::scratch_kernel(
                            disp_x,
                            disp_y,
                            half_len.max(1e-6),
                            scratch_angle.cos(),
                            scratch_angle.sin(),
                            bend,
                            thickness,
                        );

                        let contribution = kernel_val * scratch_intensity;
                        if contribution > max_val {
                            max_val = contribution;
                        }
                    }
                }

                max_val.clamp(0.0, 1.0)
            })
        }).collect();

        // No normalization — intensity directly controls brightness
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
#[path = "scratches_tests.rs"]
mod tests;
