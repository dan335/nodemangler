//! Hydraulic erosion simulation node.
//!
//! Droplet-based hydraulic erosion — the particle method of Hans Theobald Beyer,
//! "Implementation of a method for hydraulic erosion" (TU München, 2015), the
//! same algorithm popularized by Sebastian Lague's implementation.
//!
//! Water droplets rain onto the terrain one at a time and are simulated
//! sequentially, so each droplet carves the actual heightmap and the next
//! droplet flows into the channel the previous ones cut — that reinforcement is
//! what turns scattered scratches into branching drainage networks. Each droplet
//! rolls downhill (its direction is the local gradient blended with its own
//! inertia), picks up sediment while it is under its slope/speed/water-dependent
//! carrying capacity, and drops sediment when it slows, over-fills, or runs
//! uphill. Erosion is spread over a small radius-weighted brush so gullies are
//! wider than one pixel and never taken below the ground actually present at a
//! cell; deposition lands bilinearly under the droplet, building sediment fans.
//!
//! The starting terrain is either a connected height guidance map or, when
//! nothing is connected, an internal torus-mapped fBm heightmap from the seed.
//! All lookups (movement, sampling, the erosion brush) wrap toroidally, so the
//! eroded output tiles seamlessly. Droplets are simulated on a single thread
//! from one seeded RNG, so the result is byte-identical across runs.
//!
//! Complements the thermal-only "erosion" noise node: thermal erosion relaxes
//! slopes uniformly, while hydraulic erosion carves directional drainage.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Gravity constant converting a height drop into droplet acceleration.
const GRAVITY: f64 = 4.0;
/// Floor on carrying capacity so droplets on near-flat ground still hold a
/// little sediment instead of dumping everything at once.
const MIN_SEDIMENT_CAPACITY: f64 = 0.01;
/// Water volume each droplet starts with.
const INITIAL_WATER: f64 = 1.0;
/// Speed each droplet starts with.
const INITIAL_SPEED: f64 = 1.0;

/// Bilinearly samples the heightmap and its gradient at a fractional position,
/// wrapping toroidally so droplets can cross the tile seam.
///
/// Returns `(height, gradient_x, gradient_y)`.
#[inline]
fn sample_height_gradient(heightmap: &[f64], w: usize, h: usize, x: f64, y: f64) -> (f64, f64, f64) {
    let xf = x.floor();
    let yf = y.floor();
    let u = x - xf;
    let v = y - yf;
    let x0 = (xf as i64).rem_euclid(w as i64) as usize;
    let y0 = (yf as i64).rem_euclid(h as i64) as usize;
    let x1 = if x0 + 1 == w { 0 } else { x0 + 1 };
    let y1 = if y0 + 1 == h { 0 } else { y0 + 1 };
    let h00 = heightmap[y0 * w + x0];
    let h10 = heightmap[y0 * w + x1];
    let h01 = heightmap[y1 * w + x0];
    let h11 = heightmap[y1 * w + x1];
    // Gradient from the bilinear surface.
    let gx = (h10 - h00) * (1.0 - v) + (h11 - h01) * v;
    let gy = (h01 - h00) * (1.0 - u) + (h11 - h10) * u;
    let height = h00 * (1.0 - u) * (1.0 - v) + h10 * u * (1.0 - v) + h01 * (1.0 - u) * v + h11 * u * v;
    (height, gx, gy)
}

/// Bilinearly samples just the height at a fractional position (wrapped).
#[inline]
fn sample_height(heightmap: &[f64], w: usize, h: usize, x: f64, y: f64) -> f64 {
    let xf = x.floor();
    let yf = y.floor();
    let u = x - xf;
    let v = y - yf;
    let x0 = (xf as i64).rem_euclid(w as i64) as usize;
    let y0 = (yf as i64).rem_euclid(h as i64) as usize;
    let x1 = if x0 + 1 == w { 0 } else { x0 + 1 };
    let y1 = if y0 + 1 == h { 0 } else { y0 + 1 };
    let h00 = heightmap[y0 * w + x0];
    let h10 = heightmap[y0 * w + x1];
    let h01 = heightmap[y1 * w + x0];
    let h11 = heightmap[y1 * w + x1];
    h00 * (1.0 - u) * (1.0 - v) + h10 * u * (1.0 - v) + h01 * (1.0 - u) * v + h11 * u * v
}

/// Operation that carves a heightmap with droplet-based hydraulic erosion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageSimulationHydraulicErosion {}

impl OpImageSimulationHydraulicErosion {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hydraulic erosion".to_string(),
            description: "Droplet-based hydraulic erosion (Beyer's particle method): rain carves branching gullies and ridges into a heightmap, leaving sediment fans at the outflows.".to_string(),
            help: "The particle/droplet hydraulic-erosion method of Hans Theobald Beyer (TU München, 2015), the same algorithm Sebastian Lague's implementation popularized. Water droplets rain onto the terrain and are simulated one after another, so each droplet carves the real heightmap and the next flows into the channel the earlier ones cut - that reinforcement is what grows scattered scratches into branching drainage networks. Every droplet rolls downhill (its direction is the local slope blended with its own inertia), speeds up on descents, scrapes up sediment while it is under its carrying capacity, and drops sediment when it slows, over-fills, or runs uphill. Erosion is spread over a small round brush so gullies are wider than a pixel, and deposition lands under the droplet to build sediment fans.\n\nThe height map input is optional: leave it unconnected and the node generates its own fBm terrain from the seed (octaves and frequency shape that fallback terrain and are ignored when a map is connected). Droplets sets how many raindrops fall - more droplets deepen and extend the gullies. Capacity scales how much sediment a droplet can carry, so higher values cut deeper channels; erosion rate and deposition rate set how fast droplets pick up and release material. Lifetime caps how many steps a droplet travels before it dries up, so longer lives carve longer drainage paths; erosion radius sets the brush width for broader or tighter gullies. Inertia makes paths straighter and more momentum-driven versus tightly gradient-following, and evaporation shortens droplet lives.\n\nBest for terrain heightmaps, weathered rock, canyon and badlands surfaces. The output tiles seamlessly and is deterministic from the seed. Complements the thermal-only \"erosion\" noise node, which relaxes slopes uniformly instead of carving directional drainage.".to_string(),
        }
    }

    /// Creates the default inputs in the simulation convention: seed and
    /// dimensions first, then the optional height guidance map, then the droplet
    /// count, the droplet-physics params, and the fallback-terrain params last.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for droplet rainfall positions and the fallback terrain."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("height map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional starting terrain to erode; when unconnected an internal fBm heightmap is generated from the seed."),
            Input::new("droplets".to_string(), Value::Integer(400000), Some(InputSettings::DragValue { clamp: Some((0.0, 4000000.0)), speed: Some(1000.0) }), None)
                .with_description("Number of raindrops simulated; more droplets deepen and extend the gully network."),
            Input::new("capacity".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((0.01, 64.0)), speed: Some(0.05) }), None)
                .with_description("Sediment-carrying capacity multiplier; higher values let droplets cut deeper channels before depositing."),
            Input::new("erosion rate".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Fraction of a droplet's free capacity eroded per step; higher values scrape material faster."),
            Input::new("deposition rate".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Fraction of excess sediment dropped per step when a droplet is over capacity; higher values build fans sooner."),
            Input::new("lifetime".to_string(), Value::Integer(100), Some(InputSettings::DragValue { clamp: Some((1.0, 512.0)), speed: Some(1.0) }), None)
                .with_description("Maximum steps a droplet travels before drying up; longer lives carve longer drainage paths that reach the valleys."),
            Input::new("erosion radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Radius of the erosion brush in pixels at a 1024px reference (scales with the output size); larger values carve wider, softer gullies."),
            Input::new("inertia".to_string(), Value::Decimal(0.05), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How much a droplet keeps its previous direction versus following the gradient; higher values give straighter paths."),
            Input::new("evaporation".to_string(), Value::Decimal(0.01), Some(InputSettings::DragValue { clamp: Some((0.0, 0.5)), speed: Some(0.001) }), None)
                .with_description("Water lost per step; higher values dry droplets out sooner for shorter, denser carving."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the fallback terrain; only used when no height map is connected."),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Base frequency of the fallback terrain; only used when no height map is connected."),
        ]
    }

    /// Creates the default output: the eroded heightmap.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling eroded grayscale heightmap, normalized to the 0-1 range."),
        ]
    }

    /// Runs the hydraulic erosion simulation.
    ///
    /// 1. Builds the starting terrain: the connected height map resampled to the
    ///    output size, or a torus-mapped fBm fallback from the seed
    /// 2. Precomputes a radius-weighted erosion brush
    /// 3. Rains `droplets` seeded droplets one at a time; each rolls downhill
    ///    with inertia, erodes over the brush while under capacity (never below
    ///    the ground present at a cell), and deposits bilinearly when over
    ///    capacity or moving uphill — carving into the terrain the earlier
    ///    droplets already cut
    /// 4. Normalizes the eroded heightmap to [0, 1]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let map_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let droplets_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let capacity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let erosion_rate_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let deposition_rate_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let lifetime_converted = convert_input(inputs, 8, ValueType::Integer, &mut input_errors);
        let radius_converted = convert_input(inputs, 9, ValueType::Integer, &mut input_errors);
        let inertia_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let evaporation_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 12, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 13, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Integer(droplets) = droplets_converted.unwrap() else { unreachable!() };
        let Value::Decimal(capacity) = capacity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(erosion_rate) = erosion_rate_converted.unwrap() else { unreachable!() };
        let Value::Decimal(deposition_rate) = deposition_rate_converted.unwrap() else { unreachable!() };
        let Value::Integer(lifetime) = lifetime_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(inertia) = inertia_converted.unwrap() else { unreachable!() };
        let Value::Decimal(evaporation) = evaporation_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let octaves = octaves.clamp(1, 16) as usize;
        let frequency = (frequency as f64).max(0.01);
        let droplets = droplets.clamp(0, 4_000_000) as usize;
        let capacity = (capacity as f64).clamp(0.01, 64.0);
        let erosion_rate = (erosion_rate as f64).clamp(0.0, 1.0);
        let deposition_rate = (deposition_rate as f64).clamp(0.0, 1.0);
        let evaporation = (evaporation as f64).clamp(0.0, 0.5);
        let inertia = (inertia as f64).clamp(0.0, 1.0);
        // Erosion radius is authored in reference pixels (at 1024px) and
        // scaled to the generator's own width/height inputs (there is no
        // source image here) so gully width is the same relative size at any
        // output resolution.
        let radius = scale_to_resolution(radius.clamp(1, 8) as f32, width as u32, height as u32).round().max(1.0) as i64;
        let lifetime = lifetime.clamp(1, 512) as usize;

        let w = width as usize;
        let h = height as usize;
        let wf = w as f64;
        let hf = h as f64;

        // 1. Starting terrain: connected guidance map or seeded fBm fallback,
        // both in [0, 1].
        let mut heightmap: Vec<f64> = if super::is_unconnected(&map_data) {
            super::fallback_terrain(seed as u32, w, h, octaves, frequency)
        } else {
            super::guidance_map_to_grid(&map_data, w, h)
        };

        // 2. Erosion brush: cell offsets within `radius` and normalized weights
        // that fall off linearly with distance, so a droplet's erosion is spread
        // over a disc and gullies come out wider than one pixel.
        let brush: Vec<(i64, i64, f64)> = {
            let r = radius;
            let rf = r as f64;
            let mut b: Vec<(i64, i64, f64)> = Vec::new();
            let mut sum = 0.0;
            for by in -r..=r {
                for bx in -r..=r {
                    let d2 = (bx * bx + by * by) as f64;
                    if d2 < rf * rf {
                        let weight = 1.0 - d2.sqrt() / rf;
                        b.push((bx, by, weight));
                        sum += weight;
                    }
                }
            }
            for e in b.iter_mut() { e.2 /= sum; }
            b
        };

        // 3. Droplet simulation. Sequential and single-threaded: each droplet
        // erodes the live heightmap, so the next one flows into the channels the
        // earlier ones cut. One seeded RNG makes rainfall reproducible.
        let inv_inertia = 1.0 - inertia;
        let mut rng = fastrand::Rng::with_seed(seed as u64);

        for _ in 0..droplets {
            let mut px = rng.f64() * wf;
            let mut py = rng.f64() * hf;
            let mut dir_x = 0.0_f64;
            let mut dir_y = 0.0_f64;
            let mut speed = INITIAL_SPEED;
            let mut water = INITIAL_WATER;
            let mut sediment = 0.0_f64;

            for _ in 0..lifetime {
                // Old cell and in-cell offset, for deposition and the brush.
                let node_x = px.floor();
                let node_y = py.floor();
                let cell_x = px - node_x;
                let cell_y = py - node_y;
                let nx0 = (node_x as i64).rem_euclid(w as i64) as usize;
                let ny0 = (node_y as i64).rem_euclid(h as i64) as usize;

                // Height and gradient at the droplet's current position.
                let (height_old, grad_x, grad_y) = sample_height_gradient(&heightmap, w, h, px, py);

                // Blend previous direction with the downhill gradient (inertia),
                // then normalize; on flat ground with no momentum, wander.
                dir_x = dir_x * inertia - grad_x * inv_inertia;
                dir_y = dir_y * inertia - grad_y * inv_inertia;
                let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
                if len > 1e-12 {
                    dir_x /= len;
                    dir_y /= len;
                } else {
                    let angle = rng.f64() * std::f64::consts::TAU;
                    dir_x = angle.cos();
                    dir_y = angle.sin();
                }

                // Move one cell along the direction, wrapping the seam.
                px += dir_x;
                if px < 0.0 { px += wf; } else if px >= wf { px -= wf; }
                py += dir_y;
                if py < 0.0 { py += hf; } else if py >= hf { py -= hf; }

                let height_new = sample_height(&heightmap, w, h, px, py);
                let delta_h = height_new - height_old;

                // Carrying capacity grows with downhill slope, speed, and water.
                let cap = (-delta_h * speed * water * capacity).max(MIN_SEDIMENT_CAPACITY);

                if sediment > cap || delta_h > 0.0 {
                    // Over capacity or moving uphill: deposit at the old cell.
                    // Uphill fills the pit (never more than the rise); otherwise
                    // shed the excess. Deposition lands bilinearly.
                    let amount = if delta_h > 0.0 {
                        delta_h.min(sediment)
                    } else {
                        (sediment - cap) * deposition_rate
                    };
                    sediment -= amount;
                    let x1 = if nx0 + 1 == w { 0 } else { nx0 + 1 };
                    let y1 = if ny0 + 1 == h { 0 } else { ny0 + 1 };
                    heightmap[ny0 * w + nx0] += amount * (1.0 - cell_x) * (1.0 - cell_y);
                    heightmap[ny0 * w + x1] += amount * cell_x * (1.0 - cell_y);
                    heightmap[y1 * w + nx0] += amount * (1.0 - cell_x) * cell_y;
                    heightmap[y1 * w + x1] += amount * cell_x * cell_y;
                } else {
                    // Under capacity: erode over the brush, never taking more
                    // than the step's own drop, and never digging a cell below
                    // the ground actually present there (so terrain stays >= 0
                    // and no runaway pits form).
                    let amount = ((cap - sediment) * erosion_rate).min(-delta_h);
                    for &(bx, by, bw) in &brush {
                        let tx = (nx0 as i64 + bx).rem_euclid(w as i64) as usize;
                        let ty = (ny0 as i64 + by).rem_euclid(h as i64) as usize;
                        let idx = ty * w + tx;
                        let weighted = amount * bw;
                        let removed = weighted.min(heightmap[idx]);
                        heightmap[idx] -= removed;
                        sediment += removed;
                    }
                }

                // Speed and water update (Beyer/Lague): the sqrt is guarded
                // because uphill steps can drive the argument negative.
                speed = (speed * speed + delta_h * GRAVITY).max(0.0).sqrt();
                water *= 1.0 - evaporation;
            }
        }

        // 4. Normalize the eroded terrain to [0, 1].
        let min_h = heightmap.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_h = heightmap.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max_h - min_h).max(1e-10);

        let mut height_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let normalized = ((heightmap[y * w + x] - min_h) / range) as f32;
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
#[path = "hydraulic_erosion_tests.rs"]
mod tests;
