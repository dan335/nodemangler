//! Smear noise image generator.
//!
//! Produces a grayscale image of broad, soft, directional wipe marks, like
//! wiped glass, smeared fingerprints, or cloth streaks. A few large, heavily
//! elongated soft-elliptical kernels are placed on a jittered grid; all share
//! a base wipe angle with per-kernel deviation, and each is slightly bowed by
//! a random curvature so strokes arc instead of running perfectly straight.
//!
//! Marks are translucent and blended additively (clamped to 1) so overlapping
//! smears deepen, the way repeated wipes build up residue. Always tiles
//! seamlessly by wrapping kernel positions at grid boundaries.

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

/// Operation that generates a smear noise image.
///
/// Places a small number of large elongated kernels at pseudo-random positions
/// across a jittered grid. Each smudge is a soft truncated-Gaussian ellipse
/// stretched 3-8x along the wipe axis, rotated to the shared base angle plus a
/// per-kernel deviation, and bowed by a random curvature term proportional to
/// the squared coordinate along the stroke. Intensity is randomized per smudge.
///
/// Uses additive blending clamped to 1: overlapping translucent smears deepen
/// rather than replacing each other, matching how wipe residue accumulates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseSmear {}

impl OpImageNoiseSmear {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "smear noise".to_string(),
            description: "Directional wipe-mark noise. Creates soft smeared streaks for wiped glass, fingerprint smears, and cloth wipe residue.".to_string(),
            help: "Sparse convolution noise tuned for wipe marks: jittered grid cells each drop one large, heavily elongated soft ellipse with a truncated Gaussian profile. Every smudge is stretched 3-8x along its wipe axis and gently bowed by a per-smudge random curvature, so strokes arc like real hand wipes instead of running dead straight. All smudges share the base angle input, with angle variation adding a per-smudge deviation of up to +/-90 degrees, letting you go from a uniform wipe direction to chaotic smearing.\n\nDensity sets how many smudge cells fit across the tile; keep it low (the default is 3) for broad overlapping marks. Scale controls stroke length relative to cell size, and the intensity sliders keep marks translucent with per-smudge brightness variation. Contributions are blended additively and clamped to 1, so crossing smears visibly deepen where they overlap.\n\nBest for wiped-glass grime, screen smudge overlays, smeared fingerprints, brushed or wiped metal residue, and roughness-map streaks that should follow a consistent wipe direction.".to_string(),
        }
    }

    /// Creates the default inputs for the smear noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for smudge placement, stretch, and curvature; change to rearrange the smears."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(3.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.1) }), None)
                .with_description("Number of smudge cells across the image; keep low for broad overlapping wipes."),
            Input::new("scale".to_string(), Value::Decimal(1.2), Some(InputSettings::DragValue { clamp: Some((0.01, 10.0)), speed: Some(0.01) }), None)
                .with_description("Base stroke length relative to cell size; larger values produce longer sweeps."),
            Input::new("scale_variation".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much smudge sizes vary from the base scale; 0 is uniform, 1 is most varied."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: None, speed: Some(1.0) }), None)
                .with_description("Base wipe direction in degrees; all smudges align to this axis."),
            Input::new("angle_variation".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Per-smudge deviation from the base angle; 0 keeps all wipes parallel, 1 allows +/-90 degrees."),
            Input::new("intensity".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Peak brightness of each smudge; keep low so overlaps deepen gradually."),
            Input::new("intensity_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much smudge brightness varies; 0 is uniform, 1 is most varied."),
            Input::new("curve".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Maximum per-smudge curvature; 0 gives straight strokes, 1 gives strongly bowed arcs."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale smudge image of soft directional wipe marks."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords, impulse index, seed, and channel.
    #[inline(always)]
    fn hash(ix: i32, iy: i32, impulse: u32, seed: u32, channel: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ impulse.wrapping_mul(2654435761)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Evaluates a single smudge kernel at a displacement from the kernel center.
    ///
    /// The displacement is rotated into the smudge's local frame so `ly` runs
    /// along the wipe axis (semi-length `radius`) and `lx` across it
    /// (semi-width `radius / stretch`). The across coordinate is bowed by
    /// `bow * ly^2 / radius` so the stroke arcs. The profile is a truncated
    /// Gaussian, `exp(-3 d^2)` rescaled to reach exactly zero at the truncation
    /// boundary, giving a very soft translucent smear with compact support.
    #[inline(always)]
    fn smudge_kernel(
        dx: f64,
        dy: f64,
        radius: f64,
        rotation: f64,
        stretch: f64,
        bow: f64,
    ) -> f64 {
        // Rotate displacement into the smudge's local frame:
        // ly runs along the wipe axis, lx across it
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        let ly = dx * cos_r + dy * sin_r;
        let lx = -dx * sin_r + dy * cos_r;

        // Bow the across coordinate by the squared along coordinate so the
        // stroke arcs instead of running straight
        let r = radius.max(1e-10);
        let lx_curved = lx + bow * (ly * ly) / r;

        // Normalized elliptical distance: long axis = radius, short = radius / stretch
        let nx = lx_curved * stretch / r;
        let ny = ly / r;
        let d_sq = nx * nx + ny * ny;
        if d_sq >= 1.0 {
            return 0.0;
        }

        // Truncated Gaussian rescaled to hit zero at the boundary
        const FLOOR: f64 = 0.049787068367863944; // exp(-3)
        ((-3.0 * d_sq).exp() - FLOOR) / (1.0 - FLOOR)
    }

    /// Generates a smear noise image from the given inputs.
    ///
    /// Divides UV space into a grid based on density. Per cell, places one
    /// smudge at a jittered position with randomized size, stretch, angle
    /// deviation, curvature, and intensity. For each pixel, sums contributions
    /// from nearby smudge kernels and clamps to 1, so overlapping translucent
    /// smears deepen.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let scale_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let scale_var_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let angle_var_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let intensity_var_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let curve_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_variation) = scale_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle_variation) = angle_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity_variation) = intensity_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(curve) = curve_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        // Snap density to an integer so the cell grid and the pixel->grid
        // mapping span the same number of cells; a fractional density leaves a
        // partial final cell at the tile edge and breaks seamless tiling
        // (mirrors voronoi_common::grid_size_from_frequency). Integer densities
        // are unchanged.
        let density = (density as f64).max(1.0).round().max(1.0);
        let scale = (scale as f64).max(0.01);
        let scale_variation = (scale_variation as f64).clamp(0.0, 1.0);
        let base_angle = (angle as f64).to_radians();
        let angle_variation = (angle_variation as f64).clamp(0.0, 1.0);
        let intensity = (intensity as f64).clamp(0.0, 1.0);
        let intensity_variation = (intensity_variation as f64).clamp(0.0, 1.0);
        let curve = (curve as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // Long semi-axis in UV space — smudges are roughly cell-length strokes.
        // Scale variation can grow the base radius up to 2x, and curvature can
        // push the bounding circle out further; add a 30% margin on top.
        let base_radius = scale / density;
        let truncation = base_radius * (1.0 + scale_variation) * 1.3;
        let grid_size = density.ceil() as i32;

        // Additive contributions clamped to 1 (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let mut sum = 0.0_f64;

                // Pixel position in grid space
                let gx = (px as f64 / w as f64) * density;
                let gy = (py as f64 / h as f64) * density;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                // Search radius in cells. Capped at 32 so an extreme
                // scale/scale_variation combination can't grow the per-pixel
                // neighbor scan to a size that stalls large renders; smudges
                // beyond 32 cells contribute negligibly at any reasonable setting.
                let search = ((truncation * density).ceil() as i32 + 1).min(32);

                for dy in -search..=search {
                    for dx in -search..=search {
                        // Wrap cell coordinates for seamless tiling
                        let cx = (cell_x + dx).rem_euclid(grid_size);
                        let cy = (cell_y + dy).rem_euclid(grid_size);

                        // Jittered position within cell (one smudge per cell)
                        let kx = (cell_x + dx) as f64 + Self::hash(cx, cy, 0, seed_u32, 0);
                        let ky = (cell_y + dy) as f64 + Self::hash(cx, cy, 0, seed_u32, 1);

                        // Displacement from pixel to smudge center (in UV space)
                        let disp_x = (gx - kx) / density;
                        let disp_y = (gy - ky) / density;
                        let dist_sq = disp_x * disp_x + disp_y * disp_y;

                        // Skip smudges outside truncation radius
                        if dist_sq > truncation * truncation {
                            continue;
                        }

                        // Per-smudge randomized parameters
                        let size_rand = Self::hash(cx, cy, 0, seed_u32, 2);
                        let smudge_radius = base_radius * (1.0 - scale_variation + scale_variation * size_rand * 2.0);

                        let int_rand = Self::hash(cx, cy, 0, seed_u32, 3);
                        let smudge_intensity = intensity * (1.0 - intensity_variation + intensity_variation * int_rand * 2.0);

                        // Shared base angle plus per-smudge deviation of up to +/-90 degrees
                        let angle_rand = Self::hash(cx, cy, 0, seed_u32, 4);
                        let rotation = base_angle + (angle_rand * 2.0 - 1.0) * angle_variation * std::f64::consts::FRAC_PI_2;

                        // Stretch 3-8x along the wipe axis
                        let stretch_rand = Self::hash(cx, cy, 0, seed_u32, 5);
                        let stretch = 3.0 + stretch_rand * 5.0;

                        // Signed per-smudge curvature
                        let bow_rand = Self::hash(cx, cy, 0, seed_u32, 6);
                        let bow = curve * (bow_rand * 2.0 - 1.0);

                        // Additive blend: overlapping translucent smears deepen
                        sum += Self::smudge_kernel(disp_x, disp_y, smudge_radius, rotation, stretch, bow)
                            * smudge_intensity;
                    }
                }

                sum.clamp(0.0, 1.0)
            })
        }).collect();

        // No min/max normalization — values are already clamped to [0,1]
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
#[path = "smear_tests.rs"]
mod tests;
