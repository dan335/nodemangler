//! Hillslope (soil-creep) diffusion simulation node.
//!
//! Nonlinear hillslope diffusion — the soil-creep transport law of Roering,
//! Kirkby & Dietrich ("Evidence for nonlinear, diffusive sediment transport
//! on hillslopes and implications for landscape morphology", Water
//! Resources Research, 1999). Linear (Culling-style) diffusion assumes
//! sediment flux is simply proportional to slope; the Roering law instead
//! scales flux by S / (1 - (S/Sc)^2), so transport accelerates smoothly and
//! diverges as the local slope S approaches the critical slope Sc — the
//! angle at which soil can no longer creep gradually and instead fails.
//!
//! The update is an explicit finite-volume scheme on the 4-connected grid:
//! each cell exchanges flux with its four neighbors, weighted by a per-edge
//! nonlinear multiplier `m = 1 / (1 - (S/Sc)^2)` (clamped to a ceiling `M`
//! near Sc), and the flux one cell sends across an edge is exactly the flux
//! its neighbor receives across that same edge, so mass is conserved
//! exactly every step regardless of creep rate or iteration count. The
//! per-edge coefficients are kept small enough that each update is a convex
//! combination of a cell and its neighbors — a maximum principle holds, so
//! smoothing is monotone with no oscillation even at the fastest allowed
//! creep rate.
//!
//! The starting terrain is either a connected height guidance map or, when
//! nothing is connected, an internal torus-mapped fBm heightmap from the
//! seed (as with hydraulic erosion). All neighbor lookups wrap toroidally,
//! so the output tiles seamlessly whenever the input does, and the update
//! is deterministic — no randomness beyond the fallback terrain's seed.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that ages a heightmap with nonlinear hillslope (soil-creep) diffusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageSimulationHillslopeDiffusion {}

impl OpImageSimulationHillslopeDiffusion {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hillslope diffusion".to_string(),
            description: "Soil creep: nonlinear diffusion that rounds crests and relaxes slopes toward a critical angle, aging rough terrain into rolling hills.".to_string(),
            help: "Nonlinear hillslope diffusion — the soil-creep transport law of Roering, Kirkby & Dietrich (1999): sediment flux is proportional to S/(1-(S/Sc)^2), so transport diverges as slope S approaches the critical slope Sc. Solved with an explicit, exactly mass-conserving finite-volume update.\n\nThe height map input is optional: leave it unconnected and the node generates its own fBm terrain from the seed (octaves and frequency shape that fallback terrain and are ignored when a map is connected). Iterations is the main driver - it sets how many diffusion steps run, so step through it to watch the terrain age from rough to rolling. Creep rate sets the diffusion strength applied per iteration; higher values age the terrain faster. Critical slope (Sc) is the rise-over-run, in normalized domain units, at which transport diverges (1.0 = 45 degrees when relief equals extent) - lower values round hills into gentler shapes sooner.\n\nRounds convex hilltops and drives steep slopes toward the critical angle - the signature of soil-mantled rolling hills. Complements hydraulic erosion (which carves channels). Wraps at the edges, so output tiles when the input tiles. Deterministic from seed.".to_string(),
        }
    }

    /// Creates the default inputs in the simulation convention: seed and
    /// dimensions first, then the optional height guidance map, then the
    /// iteration count, the diffusion-physics params, and the fallback-terrain
    /// params last.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the fallback terrain."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("height map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional terrain to diffuse; when unconnected an internal fBm heightmap is generated from the seed."),
            Input::new("iterations".to_string(), Value::Integer(500), Some(InputSettings::DragValue { clamp: Some((0.0, 2000.0)), speed: Some(10.0) }), None)
                .with_description("Number of diffusion steps simulated; more iterations round crests and relax slopes further toward the critical angle - step through it to watch the terrain age."),
            Input::new("creep rate".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Diffusion strength applied per iteration; higher values age the terrain faster."),
            Input::new("critical slope".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 4.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Critical slope Sc, rise-over-run in normalized domain units (1.0 = 45 degrees when relief equals extent); transport diverges as the local slope approaches this value."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the fallback terrain; only used when no height map is connected."),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Base frequency of the fallback terrain; only used when no height map is connected."),
        ]
    }

    /// Creates the default output: the diffused heightmap.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling diffused grayscale heightmap, normalized to the 0-1 range."),
        ]
    }

    /// Runs the hillslope diffusion simulation.
    ///
    /// 1. Builds the starting terrain: the connected height map resampled to
    ///    the output size, or a torus-mapped fBm fallback from the seed
    /// 2. Iterates the explicit nonlinear diffusion update `iterations` times:
    ///    each cell exchanges flux with its 4 torus-wrapped neighbors,
    ///    weighted by a per-edge multiplier that grows without bound as the
    ///    local slope approaches the critical slope, using a Jacobi
    ///    (old-buffer-read, new-buffer-write) double-buffered update so every
    ///    cell updates simultaneously from a consistent snapshot
    /// 3. Normalizes the diffused heightmap to [0, 1]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let map_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let iterations_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let creep_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let critical_slope_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Integer(iterations) = iterations_converted.unwrap() else { unreachable!() };
        let Value::Decimal(creep) = creep_converted.unwrap() else { unreachable!() };
        let Value::Decimal(critical_slope) = critical_slope_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let iterations = iterations.clamp(0, 2000) as usize;
        let creep = (creep as f64).clamp(0.0, 1.0);
        let sc = (critical_slope as f64).clamp(0.1, 4.0);
        let octaves = octaves.clamp(1, 16) as usize;
        let frequency = (frequency as f64).max(0.01);

        let w = width as usize;
        let h = height as usize;

        // 1. Starting terrain: connected guidance map or seeded fBm fallback,
        // both in [0, 1].
        let mut heights: Vec<f64> = if super::is_unconnected(&map_data) {
            super::fallback_terrain(seed as u32, w, h, octaves, frequency)
        } else {
            super::guidance_map_to_grid(&map_data, w, h)
        };

        // Nonlinear multiplier ceiling near the critical slope, and the
        // per-iteration diffusion coefficient. Coefficients alpha*m are
        // always >= 0 and sum to at most 4*alpha*M = creep <= 1, so every
        // update is a convex combination of a cell and its four neighbors -
        // a maximum principle holds (monotone smoothing, no oscillation at
        // any creep rate), and `m` is symmetric per edge so mass is
        // conserved exactly.
        const M: f64 = 10.0;
        let dx = 1.0 / w.max(h) as f64;
        let alpha = creep / (4.0 * M);

        // 2. Explicit nonlinear diffusion, Jacobi double-buffered so every
        // cell reads a consistent snapshot of its neighbors.
        for _ in 0..iterations {
            let heights_ref = &heights;
            let new_heights: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |y| {
                (0..w).map(move |x| {
                    let h_i = heights_ref[y * w + x];
                    let xi = x as i64;
                    let yi = y as i64;
                    let mut lap = 0.0f64;
                    for (nxi, nyi) in [(xi + 1, yi), (xi - 1, yi), (xi, yi + 1), (xi, yi - 1)] {
                        let nx = nxi.rem_euclid(w as i64) as usize;
                        let ny = nyi.rem_euclid(h as i64) as usize;
                        let h_j = heights_ref[ny * w + nx];
                        let dh = h_j - h_i;
                        // Signed slope along this edge and the nonlinear
                        // transport multiplier; the same branch handles both
                        // the blow-up as S approaches Sc and the negative
                        // denominator when S exceeds Sc (the naive formula
                        // would anti-diffuse there).
                        let s = dh / dx;
                        let xs = (s / sc) * (s / sc);
                        let m = if xs >= 1.0 - 1.0 / M { M } else { 1.0 / (1.0 - xs) };
                        lap += m * dh;
                    }
                    h_i + alpha * lap
                })
            }).collect();
            heights = new_heights;
        }

        // 3. Normalize the diffused terrain to [0, 1] (flat terrain -> mid gray).
        let min_h = heights.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_h = heights.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_h - min_h;

        let mut height_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let normalized: f32 = if range < 1e-12 {
                    0.5
                } else {
                    ((heights[y * w + x] - min_h) / range) as f32
                };
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(normalized);
                height_image.put_pixel(x as u32, y as u32, &[non_linear]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "hillslope_diffusion_tests.rs"]
mod tests;
