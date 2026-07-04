//! Creased noise image generator.
//!
//! Produces a seamlessly tiling grayscale image of flat shaded facets that
//! meet at sharp creases, like crumpled paper or foil that has been flattened
//! back out. Each wrapped Voronoi cell becomes one facet: a planar gradient
//! along a random per-cell direction, so neighboring cells shade differently
//! and their shared border reads as a fold line.
//!
//! Extra facet layers (doubled cell count, halved amplitude) break up large
//! flat polygons, a low-amplitude periodic fbm ripple adds paper texture, and
//! an optional crease darkening term draws thin dark fold lines along cell
//! borders using the F2 - F1 distance. Always tiles seamlessly via wrapped
//! cells and integer fbm periods.

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
use noise::permutationtable::PermutationTable;

use super::voronoi_common::{cell_hash, grid_size_from_frequency, pixel_to_grid, wrap_cell};
use super::{build_perm_tables, periodic_perlin_2d};

/// Smoothstep interpolation between two edges.
#[inline(always)]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Low-amplitude periodic fbm ripple used to break up flat facets.
///
/// Sums three octaves of periodic Perlin noise with integer periods
/// (doubling per octave) so the ripple tiles seamlessly.
/// Returns a value in approximately [-1, 1].
#[inline]
fn ripple_fbm(u: f64, v: f64, base_period: isize, hashers: &[PermutationTable]) -> f64 {
    let mut result = 0.0;
    let mut amplitude = 1.0;
    let mut norm = 0.0;
    let mut period = base_period.max(1);

    for hasher in hashers {
        result += periodic_perlin_2d(u * period as f64, v * period as f64, period, period, hasher) * amplitude;
        norm += amplitude;
        amplitude *= 0.5;
        period *= 2;
    }

    result / norm
}

/// Operation that generates a creased/crumpled facet noise image.
///
/// For each pixel and facet layer, finds the nearest wrapped Voronoi site and
/// shades the pixel with a planar gradient along that cell's random direction:
/// `0.5 + fold * (dot(p - site, cell_dir) + offset)`. Because every cell has
/// its own direction and offset, adjacent facets meet in sharp value jumps —
/// the creases. Additional layers with doubled cell counts and halved
/// amplitudes add smaller facets, a periodic fbm ripple adds surface texture,
/// and crease darkening draws dark fold lines where F2 - F1 is small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCreased {}

impl OpImageNoiseCreased {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "creased noise".to_string(),
            description: "Crumpled paper/foil facets: flat shaded Voronoi cells meeting at sharp creases. Great for paper, leather crumple, and hammered metal.".to_string(),
            help: "Faceted Voronoi shading: each wrapped cell picks a random unit direction and offset from its hash, and every pixel in the cell is shaded by a planar gradient along that direction — 0.5 + fold * dot(p - site, cell_dir). Since neighboring cells shade along different directions, their shared borders appear as sharp value discontinuities, exactly like the fold lines of crumpled-then-flattened paper or foil.\n\nScale sets how many facets fit across the tile and fold controls facet contrast (how steep each planar gradient is). Layers stacks smaller facets on top (each layer doubles the cell count and halves the amplitude) so large polygons do not read as flat plastic. Ripple adds a low-amplitude periodic fbm undulation for paper-grain softness. Crease darkening draws thin dark fold lines along cell borders using the F2 - F1 border distance; 0 disables them.\n\nBest for crumpled paper, foil, leather crumple, hammered or beaten metal height maps, and as a normal-map source via the normal from height node.".to_string(),
        }
    }

    /// Creates the default inputs for the creased noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for facet layout and shading directions; change to re-crumple."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("scale".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.1) }), None)
                .with_description("Number of facet cells across the tile; higher values give smaller, busier creases."),
            Input::new("fold".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Facet contrast: how steeply each facet is shaded; 0 is flat gray, 1 is strong folds."),
            Input::new("ripple".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Low-amplitude fbm undulation added on top of the facets for paper-grain softness."),
            Input::new("crease_darkening".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Darkens thin lines along facet borders; 0 disables fold lines, 1 is strongest."),
            Input::new("layers".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of facet layers stacked; each layer doubles cell count and halves amplitude."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of flat shaded facets meeting at sharp creases."),
        ]
    }

    /// Generates a creased noise image from the given inputs.
    ///
    /// For each pixel and layer:
    /// 1. Searches the 3x3 wrapped-cell neighborhood for the nearest site
    ///    (tracking F2 - F1 on the base layer for crease darkening)
    /// 2. Shades the pixel with a planar gradient along the winning cell's
    ///    random direction, scaled by fold
    ///
    /// Layer facets are combined by a normalized weighted sum, the fbm ripple
    /// is added, borders are darkened, and the result is clamped to [0, 1].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let fold_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let ripple_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let darkening_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let layers_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(fold) = fold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ripple) = ripple_converted.unwrap() else { unreachable!() };
        let Value::Decimal(crease_darkening) = darkening_converted.unwrap() else { unreachable!() };
        let Value::Integer(layers) = layers_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let scale = (scale as f64).max(1.0);
        let fold = (fold as f64).clamp(0.0, 1.0);
        let ripple = (ripple as f64).clamp(0.0, 1.0);
        let crease_darkening = (crease_darkening as f64).clamp(0.0, 1.0);
        let layers = (layers as usize).clamp(1, 3);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let base_grid = grid_size_from_frequency(scale);

        // Ripple fbm: base period is 2x the cell grid so the undulation is
        // finer than the facets.
        let fbm_period = (base_grid as isize) * 2;
        let perm_tables = build_perm_tables(seed_u32.wrapping_add(4321), 3);
        let perm_ref = &perm_tables;

        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let mut total = 0.0_f64;
                let mut norm = 0.0_f64;
                let mut amplitude = 1.0_f64;
                // F2 - F1 border distance on the base layer, for crease darkening.
                let mut base_border = f64::MAX;

                for layer in 0..layers {
                    // Each layer doubles the cell count and uses a different seed offset.
                    let grid_size = grid_size_from_frequency(scale * (1 << layer) as f64);
                    let layer_seed = seed_u32.wrapping_add(layer as u32 * 7919);

                    let gx = pixel_to_grid(px, w, grid_size);
                    let gy = pixel_to_grid(py, h, grid_size);

                    let cell_x = gx.floor() as i32;
                    let cell_y = gy.floor() as i32;

                    let mut f1 = f64::MAX;
                    let mut f2 = f64::MAX;
                    let mut win_cx = 0;
                    let mut win_cy = 0;
                    let mut win_dx = 0.0;
                    let mut win_dy = 0.0;

                    // Search 3x3 wrapped neighborhood for the nearest site,
                    // remembering the winning cell and the displacement to it.
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let nx = wrap_cell(cell_x + dx, grid_size);
                            let ny = wrap_cell(cell_y + dy, grid_size);

                            let site_x = (cell_x + dx) as f64 + cell_hash(nx, ny, layer_seed, 0);
                            let site_y = (cell_y + dy) as f64 + cell_hash(nx, ny, layer_seed, 1);

                            let dist_x = gx - site_x;
                            let dist_y = gy - site_y;
                            let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();

                            if dist < f1 {
                                f2 = f1;
                                f1 = dist;
                                win_cx = nx;
                                win_cy = ny;
                                win_dx = dist_x;
                                win_dy = dist_y;
                            } else if dist < f2 {
                                f2 = dist;
                            }
                        }
                    }

                    if layer == 0 {
                        base_border = f2 - f1;
                    }

                    // Planar facet: shade along the cell's random unit direction,
                    // shifted by a random per-cell offset so facets differ in tone.
                    let dir_angle = cell_hash(win_cx, win_cy, layer_seed, 2) * std::f64::consts::TAU;
                    let offset = cell_hash(win_cx, win_cy, layer_seed, 3) - 0.5;
                    let dot = win_dx * dir_angle.cos() + win_dy * dir_angle.sin();
                    let facet = 0.5 + fold * (dot + offset * 0.4);

                    total += facet * amplitude;
                    norm += amplitude;
                    amplitude *= 0.5;
                }

                let mut value = total / norm;

                // Low-amplitude periodic fbm ripple for paper-grain softness.
                let u = px as f64 / w as f64;
                let v = py as f64 / h as f64;
                value += ripple * 0.15 * ripple_fbm(u, v, fbm_period, perm_ref);

                // Thin dark fold lines where the base layer's F2 - F1 is small.
                value *= 1.0 - crease_darkening * (1.0 - smoothstep(0.0, 0.12, base_border));

                value.clamp(0.0, 1.0)
            })
        }).collect();

        // Build a single-channel FloatImage from the computed pixel values
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
#[path = "creased_tests.rs"]
mod tests;
