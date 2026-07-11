//! Craters noise image generator.
//!
//! Produces a grayscale heightmap of scattered impact craters: parabolic bowls
//! with raised rims. At large scale this makes moon and asteroid surfaces; at
//! small scale it doubles as pores, pitting, and corrosion damage.
//!
//! Within an octave, overlapping craters compose by deepest-bowl and
//! highest-rim (not by summing), so clusters of impacts stay readable instead
//! of clipping to black. Octaves of smaller, weaker craters then add on top.
//! Always tiles seamlessly by wrapping cell coordinates at grid boundaries.

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

/// Operation that generates a craters heightmap image.
///
/// Places at most one crater per jittered grid cell (controlled by coverage),
/// each with randomized position, radius, and depth. A crater is a parabolic
/// bowl (`depth * (d^2 - 1)` inside the rim) plus a Gaussian rim ridge peaking
/// at the bowl edge. Within an octave overlapping craters take the deepest
/// bowl and the highest rim; octaves then sum onto a 0.5 base height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCraters {}

impl OpImageNoiseCraters {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "craters".to_string(),
            description: "Scattered impact craters: parabolic bowls with raised rims. Creates moon/asteroid heightmaps, pitting, pores, and corrosion damage.".to_string(),
            help: "Each jittered grid cell drops at most one crater (coverage sets the probability): a parabolic bowl that reaches its deepest point at the center, surrounded by a Gaussian rim ridge that peaks at the bowl edge and decays outward. Where craters of the same scale overlap, the deepest bowl and the highest rim win - like a newer impact stamped over older ground - so clusters stay readable instead of digging to black.\n\nDensity sets how many crater cells fit across the tile; size is the bowl radius relative to a cell. Octaves layer half-size, half-strength craters on top for realistic impact-age distributions.\n\nOutput is a heightmap: feed it into normal from height, ao from height, or curvature for full PBR moon, asteroid, pitted metal, or porous surfaces.".to_string(),
        }
    }

    /// Creates the default inputs for the craters operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for crater placement and sizes; change to rearrange the craters."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.1) }), None)
                .with_description("Number of crater cells across the image; higher values pack more craters."),
            Input::new("size".to_string(), Value::Decimal(0.7), Some(InputSettings::DragValue { clamp: Some((0.05, 2.0)), speed: Some(0.01) }), None)
                .with_description("Base bowl radius relative to cell size; larger values make bigger craters."),
            Input::new("size_variation".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much crater sizes vary from the base size; 0 is uniform, 1 is most varied."),
            Input::new("depth".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How deep each bowl digs below the base height."),
            Input::new("rim_height".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Height of the raised ridge around each bowl edge."),
            Input::new("rim_width".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.05, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Width of the rim ridge relative to the bowl radius; wider rims fade out further."),
            Input::new("coverage".to_string(), Value::Decimal(0.7), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Probability that a cell contains a crater; lower values leave open plains."),
            Input::new("octaves".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 6.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of crater scales layered; more octaves add smaller impacts on top."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale heightmap of impact craters around mid-gray."),
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

    /// Evaluates one crater's height contribution at normalized distance `d`
    /// from its center (`d` = 1 at the bowl edge). Returns `(bowl, rim)`.
    ///
    /// Inside the rim the bowl is a parabola from `-depth` at the center to 0
    /// at the edge; a Gaussian ridge of height `rim_height` and relative width
    /// `rim_width` peaks at the edge and decays outward. Bowl and rim are
    /// returned separately so overlapping craters can compose by deepest bowl
    /// and highest rim.
    #[inline(always)]
    fn crater_profile(d: f64, depth: f64, rim_height: f64, rim_width: f64) -> (f64, f64) {
        // Parabolic bowl, zero at and beyond the rim
        let bowl = if d < 1.0 { depth * (d * d - 1.0) } else { 0.0 };

        // Gaussian rim ridge centered on the bowl edge
        let rd = (d - 1.0) / rim_width;
        let rim = rim_height * (-rd * rd).exp();

        (bowl, rim)
    }

    /// Generates a craters heightmap image from the given inputs.
    ///
    /// For each octave, divides UV space into a grid based on density (doubling
    /// each octave, with half-strength craters). Per cell, drops a crater with
    /// probability `coverage` at a jittered position with randomized radius.
    /// Within an octave the deepest bowl and highest rim win; octave
    /// contributions then sum onto a 0.5 base height, clamped to [0, 1].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let size_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let size_var_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let depth_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let rim_height_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let rim_width_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let coverage_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 10, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(size) = size_converted.unwrap() else { unreachable!() };
        let Value::Decimal(size_variation) = size_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(depth) = depth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rim_height) = rim_height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rim_width) = rim_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(coverage) = coverage_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        // Snap density to an integer so each octave's grid (oct_density =
        // density * 2^octave) and its pixel->grid mapping span the same integer
        // number of cells; a fractional density leaves a partial final cell at
        // the tile edge and breaks seamless tiling (mirrors
        // voronoi_common::grid_size_from_frequency). Integer densities are unchanged.
        let density = (density as f64).max(1.0).round().max(1.0);
        let size = (size as f64).clamp(0.05, 2.0);
        let size_variation = (size_variation as f64).clamp(0.0, 1.0);
        let depth = (depth as f64).clamp(0.0, 1.0);
        let rim_height = (rim_height as f64).clamp(0.0, 1.0);
        let rim_width = (rim_width as f64).clamp(0.05, 1.0);
        let coverage = (coverage as f64).clamp(0.0, 1.0);
        let octaves = (octaves as usize).clamp(1, 6);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // A crater's influence ends where the rim Gaussian becomes negligible
        // (3 rim widths past the bowl edge), for the largest possible radius.
        let max_radius = size * (1.0 + size_variation);
        let extent = max_radius * (1.0 + 3.0 * rim_width);

        // Sum crater contributions from all octaves onto a 0.5 base (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let mut height_val = 0.5_f64;

                for octave in 0..octaves {
                    // Each octave doubles density and halves crater strength
                    let oct_density = density * (1 << octave) as f64;
                    let oct_strength = 1.0 / (1 << octave) as f64;
                    let oct_seed = seed_u32.wrapping_add(octave as u32 * 7919);
                    let grid_size = oct_density.ceil() as i32;

                    // Pixel position in this octave's grid space (cell units)
                    let gx = (px as f64 / w as f64) * oct_density;
                    let gy = (py as f64 / h as f64) * oct_density;

                    let cell_x = gx.floor() as i32;
                    let cell_y = gy.floor() as i32;

                    // Crater extent in cells is scale-invariant (radius is relative to cell size)
                    let search = extent.ceil() as i32 + 1;

                    // Deepest bowl and highest rim among this octave's craters
                    let mut oct_bowl = 0.0_f64;
                    let mut oct_rim = 0.0_f64;

                    for dy in -search..=search {
                        for dx in -search..=search {
                            // Wrap cell coordinates for seamless tiling
                            let cx = (cell_x + dx).rem_euclid(grid_size);
                            let cy = (cell_y + dy).rem_euclid(grid_size);

                            // Coverage: not every cell holds a crater
                            if Self::hash(cx, cy, oct_seed, 9) >= coverage {
                                continue;
                            }

                            // Crater center jittered within its cell
                            let kx = (cell_x + dx) as f64 + Self::hash(cx, cy, oct_seed, 0);
                            let ky = (cell_y + dy) as f64 + Self::hash(cx, cy, oct_seed, 1);

                            let disp_x = gx - kx;
                            let disp_y = gy - ky;
                            let dist = (disp_x * disp_x + disp_y * disp_y).sqrt();

                            // Per-crater randomized radius
                            let size_rand = Self::hash(cx, cy, oct_seed, 2);
                            let radius = size * (1.0 - size_variation + size_variation * size_rand * 2.0);

                            let d = dist / radius.max(1e-10);
                            if d > 1.0 + 3.0 * rim_width {
                                continue;
                            }

                            let (bowl, rim) = Self::crater_profile(d, depth, rim_height, rim_width);
                            oct_bowl = oct_bowl.min(bowl);
                            oct_rim = oct_rim.max(rim);
                        }
                    }

                    height_val += oct_strength * (oct_bowl + oct_rim);
                }

                height_val.clamp(0.0, 1.0)
            })
        }).collect();

        // No normalization — craters displace a fixed 0.5 base height
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
#[path = "craters_tests.rs"]
mod tests;
