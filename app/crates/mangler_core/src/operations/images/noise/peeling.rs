//! Peeling noise image generator.
//!
//! Produces a seamlessly tiling grayscale mask of peeling/flaking paint. A
//! low-frequency periodic fBm coverage field decides the large-scale
//! geography — big connected continents of intact paint and big connected
//! bare patches — while a domain-warped, size-weighted wrapped-cell Voronoi
//! layer breaks the boundary between them into individual flakes. Failure is
//! quantized per flake: each flake samples the coverage field at its own site
//! and pops off whole, so the intact/bare boundary is jagged at flake
//! granularity instead of following a smooth noise contour. Rare hash-flipped
//! stragglers add isolated surviving flakes deep in the bare zone and small
//! bare potholes inside the continents.
//!
//! Kept flakes are eroded inward with high-frequency periodic fBm chipping
//! (strongest for flakes near the failure boundary) and shaded with a bright
//! rim just inside the edge — the curled, lifted lip of the flake catching
//! light. Always tiles seamlessly via wrapped cells and integer fBm periods.

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

use super::voronoi_common::{cell_hash, grid_size_from_frequency, wrap_cell};
use super::{build_perm_tables, periodic_perlin_2d};

/// Antialiasing width of the flake edge, in cell units. Small so edges stay
/// fairly hard with just a hint of smoothing.
const EDGE_WIDTH: f64 = 0.03;

/// Integer period (lattice cells across the tile) of the low-frequency
/// coverage field that lays out intact continents vs. bare patches.
const COVERAGE_PERIOD: isize = 3;

/// Smoothstep interpolation between two edges.
#[inline(always)]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Periodic fBm: sums one octave of periodic Perlin noise per supplied
/// permutation table, with integer periods doubling per octave so the sum
/// tiles seamlessly. Returns a value in approximately [-1, 1].
#[inline]
fn periodic_fbm(u: f64, v: f64, base_period: isize, hashers: &[PermutationTable]) -> f64 {
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

/// Operation that generates a peeling/flaking-paint mask image.
///
/// A low-frequency coverage fBm decides where paint survives at continent
/// scale. Pixels are domain-warped by medium-frequency fBm and assigned to
/// the nearest size-weighted wrapped Voronoi site, so flakes vary in size and
/// shape instead of reading as uniform cells. Each flake then fails or
/// survives as a whole: the coverage field is sampled at the flake's site,
/// jittered by a per-flake hash, and compared against the coverage threshold,
/// which quantizes the intact/bare boundary to flake-shaped jags. A small
/// per-flake hash chance flips the decision for isolated stragglers. Kept
/// flakes are chipped inward with high-frequency fBm erosion — deeper for
/// flakes near the failure boundary — and shaded with a bright rim just
/// inside the edge for the curled lifted lip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoisePeeling {}

impl OpImageNoisePeeling {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "peeling noise".to_string(),
            description: "Peeling-paint mask: large intact continents and bare patches with a flake-quantized boundary, chipped edges, and curled bright rims.".to_string(),
            help: "Two fields combine into the mask. A low-frequency periodic fBm coverage field lays out the geography: large connected regions of intact paint (white) and large connected bare regions (black). On top of it, a wrapped-cell Voronoi layer — domain-warped by medium-frequency fBm and with per-site size weights so cells vary widely in size and shape — divides the surface into individual flakes. Failure is decided per flake, not per pixel: each flake samples the coverage field at its own site, adds a per-flake hash jitter, and compares against the coverage threshold. Whole flakes pop off along the boundary, so the transition between intact and bare is jagged at flake granularity, and a small hash chance flips isolated flakes far from the boundary — lone survivors in the bare zone, small potholes in the continents.\n\nScale sets how many flakes fit across the tile; the continents themselves stay a few times larger. Coverage moves the failure threshold: 1 is fully painted, 0 is fully stripped, and values between grow or shrink the bare regions flake by flake. Distortion controls the domain warp that hides the underlying cell structure. Roughness chips the surviving flake edges inward with high-frequency fBm — flakes close to the failure boundary are eaten hardest, so the coastline looks progressively more decayed. Curl shades each flake with a bright rim just inside its edge (the lifted lip catching light) over a slightly darker flake body; 0 keeps flakes flat white.\n\nBest used as a mask blending intact paint over exposed substrate: peeling paint, chipped enamel, flaking rust coatings, and worn decals.".to_string(),
        }
    }

    /// Creates the default inputs for the peeling noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the coverage geography, flake layout, and which flakes survive."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("scale".to_string(), Value::Decimal(14.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.1) }), None)
                .with_description("Number of flake cells across the tile; higher values produce smaller, denser flakes."),
            Input::new("coverage".to_string(), Value::Decimal(0.55), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Fraction of paint remaining; 1 is fully painted, 0 is fully stripped bare."),
            Input::new("distortion".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Domain-warp strength on the flake lookup; higher values bend flakes into irregular organic shapes."),
            Input::new("roughness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How deeply flake edges are chipped by high-frequency noise; boundary flakes are eaten hardest."),
            Input::new("curl".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Strength of the bright lifted-lip rim just inside each flake edge; 0 keeps flakes flat white."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale peeling-paint mask: intact paint near white, bare substrate black."),
        ]
    }

    /// Generates a peeling noise mask from the given inputs.
    ///
    /// For each pixel:
    /// 1. Domain-warps the grid coordinates with medium-frequency periodic fBm
    /// 2. Finds the nearest size-weighted wrapped Voronoi site (5x5 search)
    /// 3. Samples the low-frequency coverage fBm at the winning site, jitters
    ///    it per flake, and keeps or drops the whole flake against the
    ///    coverage threshold (with a rare hash flip for stragglers)
    /// 4. Erodes kept flakes inward with high-frequency fBm chipping, deeper
    ///    for flakes near the failure boundary
    /// 5. Shades a bright curl rim just inside the flake edge
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let coverage_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let distortion_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let roughness_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let curl_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(coverage) = coverage_converted.unwrap() else { unreachable!() };
        let Value::Decimal(distortion) = distortion_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roughness) = roughness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(curl) = curl_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let scale = (scale as f64).max(1.0);
        let coverage = (coverage as f64).clamp(0.0, 1.0);
        let distortion = (distortion as f64).clamp(0.0, 1.0);
        let roughness = (roughness as f64).clamp(0.0, 1.0);
        let curl = (curl as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let grid_size = grid_size_from_frequency(scale);
        let grid_f = grid_size as f64;

        // Coverage threshold: stretched slightly past [0, 1] so the extremes
        // of the slider reach fully painted / fully bare despite the fBm
        // rarely hitting its theoretical range.
        let threshold = 0.5 + (0.5 - coverage) * 1.4;

        // Low-frequency coverage field: continents of intact paint. Three
        // octaves keep the geography coherent — more would fragment the
        // continents into camouflage blobs.
        let coverage_tables = build_perm_tables(seed_u32.wrapping_add(1013), 3);
        // Medium-frequency domain warp, one table set per axis.
        let warp_period = ((grid_size / 2) as isize).max(2);
        let warp_x_tables = build_perm_tables(seed_u32.wrapping_add(4241), 2);
        let warp_y_tables = build_perm_tables(seed_u32.wrapping_add(7333), 2);
        // High-frequency chipping fBm, finer than the flakes themselves.
        let ragged_period = (grid_size as isize) * 6;
        let ragged_tables = build_perm_tables(seed_u32.wrapping_add(9173), 3);

        // Warp amplitude in cell units: up to ~0.8 cells at full distortion.
        let warp_amp = distortion * 0.8;

        let coverage_ref = &coverage_tables;
        let warp_x_ref = &warp_x_tables;
        let warp_y_ref = &warp_y_tables;
        let ragged_ref = &ragged_tables;

        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let u = px as f64 / w as f64;
                let v = py as f64 / h as f64;

                // Domain-warp the flake lookup so cells vary in apparent
                // size and shape instead of reading as textbook Voronoi.
                let gx = u * grid_f + warp_amp * periodic_fbm(u, v, warp_period, warp_x_ref);
                let gy = v * grid_f + warp_amp * periodic_fbm(u, v, warp_period, warp_y_ref);

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                let mut f1 = f64::MAX;
                let mut f2 = f64::MAX;
                let mut win_cx = 0;
                let mut win_cy = 0;
                let mut win_site_x = 0.0;
                let mut win_site_y = 0.0;

                // Search the 5x5 wrapped neighborhood for the two nearest
                // size-weighted sites (5x5 because per-site weights let a
                // large flake reach past its immediate neighbors).
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        let nx = wrap_cell(cell_x + dx, grid_size);
                        let ny = wrap_cell(cell_y + dy, grid_size);

                        let site_x = (cell_x + dx) as f64 + cell_hash(nx, ny, seed_u32, 0);
                        let site_y = (cell_y + dy) as f64 + cell_hash(nx, ny, seed_u32, 1);

                        // Per-site size weight: smaller weights make a site
                        // win a larger region, adding flake size variety.
                        let weight = 0.72 + 0.56 * cell_hash(nx, ny, seed_u32, 2);

                        let dist_x = gx - site_x;
                        let dist_y = gy - site_y;
                        let dist = (dist_x * dist_x + dist_y * dist_y).sqrt() * weight;

                        if dist < f1 {
                            f2 = f1;
                            f1 = dist;
                            win_cx = nx;
                            win_cy = ny;
                            win_site_x = site_x;
                            win_site_y = site_y;
                        } else if dist < f2 {
                            f2 = dist;
                        }
                    }
                }

                // Quantized per-flake failure: sample the coverage field at
                // the flake's site, jitter it per flake, and compare against
                // the threshold — the whole flake pops off or survives.
                let site_u = win_site_x / grid_f;
                let site_v = win_site_y / grid_f;
                let b_site = 0.5 + 1.0 * periodic_fbm(site_u, site_v, COVERAGE_PERIOD, coverage_ref);
                let jitter = (cell_hash(win_cx, win_cy, seed_u32, 5) - 0.5) * 0.18;
                let mut margin = b_site + jitter - threshold;

                // Rare stragglers: flip isolated flakes far from the boundary
                // (lone survivors in the bare zone, potholes in continents).
                if cell_hash(win_cx, win_cy, seed_u32, 11) < 0.045 {
                    if margin > 0.0 {
                        margin = -1.0;
                    } else {
                        // Flipped survivors read as heavily decayed leftovers.
                        margin = 0.03 + 0.08 * cell_hash(win_cx, win_cy, seed_u32, 13);
                    }
                }

                if margin <= 0.0 {
                    return 0.0;
                }

                // Distance from this pixel to the flake border, in cell units.
                let d_border = (f2 - f1) * 0.5;

                // Chip the border inward: flakes near the failure boundary
                // are eaten hardest, and high-frequency fBm makes the
                // chipping ragged. Failure proximity combines the flake's own
                // margin with the pixel's distance to the geographic
                // boundary, so even a strong flake gets chipped along the
                // side that faces bare substrate — otherwise continent coasts
                // show long clean Voronoi walls. Deep-interior flakes fuse
                // into solid paint (the negative base extends them past their
                // borders) with only occasional hairline cracks.
                let b_pixel = 0.5 + 1.0 * periodic_fbm(u, v, COVERAGE_PERIOD, coverage_ref);
                let pixel_margin = b_pixel - threshold + 0.09;
                let ragged = periodic_fbm(u, v, ragged_period, ragged_ref);
                let edge_fail = 1.0 - smoothstep(0.0, 0.22, margin.min(pixel_margin));
                let eat = -0.08 + 0.30 * edge_fail
                    + roughness * (0.10 + 0.14 * edge_fail) * ragged;

                let d_eff = d_border - eat;
                let mask = smoothstep(0.0, EDGE_WIDTH, d_eff);

                // Curl: bright rim just inside the flake edge (the lifted lip
                // catching light) over a slightly darker flake body. Gated by
                // edge_fail so lips only lift where the paint is failing —
                // deep-interior paint stays solid flat white.
                let rim = 1.0 - smoothstep(EDGE_WIDTH, 0.12, d_eff);
                let lip = curl * (0.15 + 0.85 * edge_fail);
                (mask * (1.0 - 0.3 * lip * (1.0 - rim))).clamp(0.0, 1.0)
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
#[path = "peeling_tests.rs"]
mod tests;
