//! Rolling hills noise image generator.
//!
//! Produces a grayscale heightmap of gentle, rounded hills: one smooth bump
//! per jittered grid cell, combined where neighboring hills overlap (a
//! merge slider blends tallest-wins into summing). At low `size` values the
//! hills stay separate mounds; above 1 they overlap into continuous rolling
//! terrain. Always tiles seamlessly by wrapping cell coordinates at grid
//! boundaries.

use rayon::prelude::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::noise::voronoi_common::{cell_hash, wrap_cell};
use crate::operations::images::tone_curve::{optional_lut, sample_lut, tone_curve_input};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a rolling hills heightmap image.
///
/// Scatters one Hann-kernel bump per jittered grid cell (the same
/// Worley-style construction as the craters node), combines overlapping
/// contributions per the merge input, then min/max normalizes to [0, 1].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseRollingHills {}

impl OpImageNoiseRollingHills {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rolling hills".to_string(),
            description: "Gentle rounded hills from overlapping smooth bumps scattered on a jittered grid.".to_string(),
            help: "Heuristic hill splatting: one smooth Hann-kernel bump per jittered grid cell, combined where hills overlap (the same Worley-style scatter construction as the craters node) - not a physical model.\n\nSeed picks the arrangement of hills. Width/height set the output resolution. Density controls how many hill cells fit across the tile (snapped to an integer so the pattern tiles). Size is each hill's radius in cell units - above 1 neighboring hills overlap into continuous rolling terrain, below 1 they stay separate mounds. Size variation randomizes each hill's radius around that base size; height variation randomizes each hill's peak amplitude. Peakiness reshapes the hill profile: below 1 gives flat-topped downs, above 1 pointier knolls. Merge sets how overlapping hills combine: 0 keeps each hill's silhouette distinct (the tallest wins), 1 sums overlaps into merged rolling terrain. Profile remaps each hill's dome through a drawn curve: x is the smooth dome's own height (0 = rim, 1 = peak); the default diagonal changes nothing, and a curve that doesn't return 0 at x=0 gives hills a cliff at their rims.\n\nTiles seamlessly. Deterministic from seed. For regional variation in hilliness, multiply the output with a low-frequency perlin or fbm noise downstream.".to_string(),
        }
    }

    /// Creates the default inputs for the rolling hills operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for hill placement and sizes; change to rearrange the hills."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(6.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.1) }), None)
                .with_description("Hills per axis; snapped to an integer grid internally so the pattern tiles."),
            Input::new("size".to_string(), Value::Decimal(1.4), Some(InputSettings::DragValue { clamp: Some((0.5, 2.5)), speed: Some(0.01) }), None)
                .with_description("Hill radius in cell units; above 1 the hills overlap into continuous rolling terrain."),
            Input::new("size_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How much hill radii vary from the base size; 0 is uniform, 1 is most varied."),
            Input::new("height_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How much each hill's peak height varies; 0 is uniform, 1 is most varied."),
            Input::new("peakiness".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.25, 4.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Hill profile exponent: below 1 gives flat-topped downs, 1 is a smooth dome, above 1 gives pointier knolls."),
            Input::new("merge".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How overlapping hills combine: 0 keeps each hill's silhouette distinct (tallest wins), 1 sums them into continuous rolling terrain."),
            tone_curve_input("profile", "Remaps each hill's dome height: x is the smooth dome's own height (0 = rim, 1 = peak). The default diagonal changes nothing; a curve that doesn't return 0 at x=0 gives hills a cliff at their rims."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale heightmap of rolling hills normalized to [0, 1]."),
        ]
    }

    /// Generates a rolling hills heightmap image from the given inputs.
    ///
    /// Per pixel, sums the Hann-kernel contribution of every hill within
    /// `size * (1 + size_variation)` cells, where a hill's center, radius,
    /// and amplitude are all derived from the hash of its (wrapped) grid
    /// cell so the pattern tiles exactly at the image edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let size_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let size_var_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let height_var_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let peakiness_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let merge_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let profile_converted = convert_input(inputs, 9, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(size) = size_converted.unwrap() else { unreachable!() };
        let Value::Decimal(size_variation) = size_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(height_variation) = height_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(peakiness) = peakiness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(merge) = merge_converted.unwrap() else { unreachable!() };
        let Value::Curve(profile) = profile_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let density = (density as f64).clamp(1.0, 32.0);
        let size = (size as f64).clamp(0.5, 2.5);
        let size_variation = (size_variation as f64).clamp(0.0, 1.0);
        let height_variation = (height_variation as f64).clamp(0.0, 1.0);
        let peakiness = (peakiness as f64).clamp(0.25, 4.0);
        let merge = (merge as f64).clamp(0.0, 1.0);

        // None when the profile is the untouched identity default: the hot
        // loop then skips the remap entirely, so the default output stays
        // bit-identical to the pre-profile behaviour at zero cost.
        let dome_lut = optional_lut(&profile);
        let dome_lut_ref: Option<&[f32]> = dome_lut.as_deref();

        // Snap density to an integer grid so the pixel->grid mapping spans an
        // exact integer number of cells; a fractional density leaves a
        // partial final cell at the tile edge and breaks seamless tiling.
        let grid = density.round().max(1.0) as i32;
        let seed_u32 = seed as u32;

        // A hill's influence ends at its own radius (Hann kernel is zero past
        // the edge), for the largest possible radius.
        let max_radius = size * (1.0 + size_variation);
        let search = max_radius.ceil() as i32 + 1;

        let w = width as usize;
        let h = height as usize;

        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let gx = (px as f64 / w as f64) * grid as f64;
                let gy = (py as f64 / h as f64) * grid as f64;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                let mut sum = 0.0_f64;
                let mut tallest = 0.0_f64;

                for dy in -search..=search {
                    for dx in -search..=search {
                        // Hash on the wrapped cell so it tiles
                        let wx = wrap_cell(cell_x + dx, grid);
                        let wy = wrap_cell(cell_y + dy, grid);

                        // Hill center jittered within its (unwrapped) cell
                        let kx = (cell_x + dx) as f64 + cell_hash(wx, wy, seed_u32, 0);
                        let ky = (cell_y + dy) as f64 + cell_hash(wx, wy, seed_u32, 1);

                        // Per-hill randomized radius, spanning size * (1 +/- variation)
                        let r = size * (1.0 - size_variation + 2.0 * size_variation * cell_hash(wx, wy, seed_u32, 2));

                        let ddx = gx - kx;
                        let ddy = gy - ky;
                        let d2 = ddx * ddx + ddy * ddy;
                        if d2 >= r * r {
                            continue;
                        }

                        let t = d2.sqrt() / r;
                        let amp = 1.0 - height_variation * cell_hash(wx, wy, seed_u32, 3);

                        // Hann kernel: C1-smooth at both the peak (t=0) and the
                        // edge (t=1). The peakiness exponent keeps those
                        // properties for any positive power. The optional
                        // profile tone curve then remaps the dome's own height.
                        // NOTE: guided rolling hills ports this splat loop
                        // (including this dome-remap hook) verbatim — keep
                        // `simulation/guided_rolling_hills.rs` in sync.
                        let mut dome = (0.5 + 0.5 * (std::f64::consts::PI * t).cos()).powf(peakiness);
                        if let Some(lut) = dome_lut_ref {
                            dome = sample_lut(lut, dome as f32) as f64;
                        }
                        let contribution = amp * dome;
                        sum += contribution;
                        tallest = tallest.max(contribution);
                    }
                }

                // Blend how overlaps combine: tallest-wins keeps silhouettes
                // distinct, summing merges hills into continuous terrain.
                tallest + merge * (sum - tallest)
            })
        }).collect();

        // Normalize to [0, 1]
        let min_val = buffer.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = buffer.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_val - min_val;

        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let normalized = if range < 1e-12 {
                    0.5_f32
                } else {
                    ((buffer[y * w + x] - min_val) / range) as f32
                };
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(normalized);
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
#[path = "rolling_hills_tests.rs"]
mod tests;
