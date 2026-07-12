//! River network simulation node.
//!
//! Physically-based drainage networks. Rather than painting river-looking
//! streaks, this node runs the standard terrain-hydrology pipeline: it fills
//! depressions with the priority-flood method (Barnes, Lehman & Mutz 2014),
//! routes water from every cell to its steepest downslope neighbor (D8,
//! O'Callaghan & Marks 1984), accumulates the upstream drainage area, and
//! declares the cells whose drainage exceeds a threshold to be channels. Rivers
//! therefore emerge exactly where the terrain would really drain, so they match
//! the surface by construction instead of being pasted on top of it.
//!
//! Channel depth follows a stream-power-style law (drainage area raised to an
//! exponent) and channel width follows hydraulic geometry (width proportional to
//! the square root of drainage area, Leopold & Maddock 1953). The valley
//! cross-section is carved from a labeled distance field — a heuristic profile,
//! not a sediment-transport model — so small streams get sharp V walls and big
//! rivers get flat U floors automatically.
//!
//! Water drains at the image border (the "ocean"), so unlike the hydraulic
//! erosion node the output is NOT tileable. Everything is deterministic from the
//! seed: the only randomness is the fBm fallback terrain used when no height map
//! is connected.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::Instant;

/// Tiny per-step height increment used by the priority-flood fill and the bed
/// re-enforcement sweep to guarantee a strictly monotone downhill path (so
/// every interior cell has at least one strictly lower neighbor to drain to).
const FILL_EPS: f64 = 1e-8;
/// How deep the optional river-guide pilot trench cuts, as a fraction of the
/// guide brightness times guide strength. Only steers routing; never appears in
/// the carved output on its own.
const PILOT_CARVE_FACTOR: f64 = 0.3;
/// Floor on a channel cell's width in pixels, so the smallest tributaries still
/// carve at least a sliver rather than vanishing between sample points.
const MIN_CHANNEL_WIDTH: f64 = 0.75;

/// The eight neighbor offsets, in a fixed scan order that deterministically
/// breaks D8 slope ties toward earlier entries.
const NEIGH8: [(i64, i64); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1, 0), (1, 0),
    (-1, 1), (0, 1), (1, 1),
];

/// A cell waiting in the priority-flood frontier. Ordered so the standard
/// (max-)`BinaryHeap` pops the LOWEST height first, with a monotonically
/// increasing push counter as a fully deterministic tie-break (`f64` is not
/// `Ord`, so `total_cmp` plus the counter give a total order).
struct HeapCell {
    /// Filled height of the cell.
    h: f64,
    /// Push order; smaller was pushed earlier.
    order: u64,
    /// Flat grid index (`y * w + x`).
    idx: u32,
}

impl PartialEq for HeapCell {
    fn eq(&self, other: &Self) -> bool {
        self.h == other.h && self.order == other.order
    }
}
impl Eq for HeapCell {}
impl Ord for HeapCell {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed so BinaryHeap (a max-heap) behaves as a min-heap on height,
        // then on push order.
        other.h.total_cmp(&self.h).then_with(|| other.order.cmp(&self.order))
    }
}
impl PartialOrd for HeapCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Encodes a linear [0, 1] value to non-linear sRGB and returns it as a single
/// pixel channel, matching the hydraulic-erosion node's output convention.
#[inline]
fn encode(v: f64) -> f32 {
    crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(v as f32)
}

/// Builds a single-channel image from a grid of already-[0, 1] linear values,
/// sRGB-encoding each pixel.
fn grid_to_image(grid: &[f64], w: usize, h: usize) -> FloatImage {
    let mut img = FloatImage::new(w as u32, h as u32, 1);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x as u32, y as u32, &[encode(grid[y * w + x])]);
        }
    }
    img
}

/// Operation that grows drainage-based river networks on a heightmap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageSimulationRivers {}

impl OpImageSimulationRivers {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rivers".to_string(),
            description: "Physically-based river networks: fills depressions, routes flow downhill, accumulates drainage area, and carves channels where the terrain would really drain.".to_string(),
            help: "Real-hydrology river networks. Depressions are filled with the priority-flood method (Barnes, Lehman & Mutz 2014), then water is routed from every cell to its single steepest downslope neighbor (D8, O'Callaghan & Marks 1984) and the upstream drainage area is accumulated. Cells whose drainage passes the river-amount threshold become channels, so rivers emerge exactly where the surface would really drain and match the terrain by construction.\n\nDepth follows a stream-power-style law (drainage area raised to the depth exponent) and width follows hydraulic geometry (width grows with the square root of drainage area, Leopold & Maddock 1953), so trunk rivers are wide and deep while headwater streams stay thin. The valley cross-section itself is a heuristic distance-field profile, not sediment transport: within a channel's width the bed is flat, then valley width and valley shape blend the walls from sharp V-shaped streams to broad U-shaped rivers.\n\nThe height map input is optional: leave it unconnected and the node builds its own fBm terrain from the seed (octaves and frequency shape that fallback and are ignored when a map is connected). The river guide input is also optional: when connected it pilot-carves a shallow channel along the bright guide pixels BEFORE routing, so rivers prefer your path while still obeying the terrain - the pilot trench only steers the flow and never shows up in the output on its own. Raise guide strength if rivers ignore the path, or reach for the carve river node when you need a river to follow an exact drawn path.\n\nRiver amount sets the network density (how much drainage counts as a river); carve depth and depth exponent set how deeply channels incise; river width is the largest river's width in pixels at a 1024px reference, so it scales with the output resolution; bed smoothing relaxes the river bed while a final sweep keeps water flowing strictly downhill.\n\nFour outputs: height is the carved terrain; river mask is the channel network (brighter for bigger rivers); flow map shows the entire drainage tree at every pixel (faint tributaries to strong trunks); water depth is the river and lake water column. Filling depressions turns enclosed basins into lakes, which appear in the water depth output for free. Water drains at the image edges, so the output does NOT tile (unlike the hydraulic erosion node). Deterministic from the seed.".to_string(),
        }
    }

    /// Creates the default inputs in the simulation convention: seed and
    /// dimensions first, then the optional guidance maps, then the main drivers,
    /// then the fine-tuning params, with the fallback-terrain params last.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the fallback terrain (the only randomness in the node)."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("height map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional starting terrain to drain; when unconnected an internal fBm heightmap is generated from the seed."),
            Input::new("river guide".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional guide: bright pixels pilot-carve a shallow channel BEFORE routing, so rivers prefer that path while still obeying the terrain."),
            Input::new("river amount".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Network density: how much upstream drainage counts as a river; higher values grow more, smaller tributaries."),
            Input::new("carve depth".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How deeply the biggest rivers incise into the terrain (stream-power scaled by drainage area)."),
            Input::new("depth exponent".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.1, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Stream-power exponent on drainage area; lower values make even small tributaries incise, higher values reserve depth for trunk rivers."),
            Input::new("river width".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None)
                .with_description("Width of the largest river in pixels at a 1024px reference (scales with the output size); tributaries are narrower by hydraulic geometry."),
            Input::new("valley width".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.05) }), None)
                .with_description("How far the valley walls extend beyond each channel, as a multiple of that channel's width."),
            Input::new("valley shape".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Valley wall profile from V-shaped (low) to U-shaped (high)."),
            Input::new("bed smoothing".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Along-flow smoothing passes on the river bed; a final sweep keeps water flowing strictly downhill."),
            Input::new("guide strength".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How strongly the river guide biases routing; raise it if rivers ignore the drawn path."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the fallback terrain; only used when no height map is connected."),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Base frequency of the fallback terrain; only used when no height map is connected."),
        ]
    }

    /// Creates the four outputs: carved height, river mask, flow map, water depth.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Carved grayscale terrain with river valleys, normalized to the 0-1 range."),
            Output::new("river mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("River channel network; brightness tracks river size (bigger rivers are brighter)."),
            Output::new("flow map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Log-scaled upstream drainage at every pixel: the whole tributary tree, faint to strong."),
            Output::new("water depth".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("River and lake water column; filled depressions show up here as lakes."),
        ]
    }

    /// Runs the river-network simulation.
    ///
    /// 1. Builds the starting terrain (connected height map or fBm fallback)
    /// 2. Optionally pilot-carves the routing terrain along a river guide
    /// 3. Fills depressions with priority-flood + epsilon drainage
    /// 4. Routes D8 flow to the steepest downslope neighbor
    /// 5. Accumulates upstream drainage area
    /// 6. Thresholds drainage into channels
    /// 7. Computes per-channel bed depth and width, then smooths the bed
    /// 8. Distance-transforms the channel network with nearest-channel labels
    /// 9. Carves valleys into the ORIGINAL terrain
    /// 10. Fills the channels and lake basins with water
    /// 11. Normalizes and emits the four outputs
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let map_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let guide_converted = convert_input(inputs, 4, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let carve_depth_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let depth_exp_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let river_width_converted = convert_input(inputs, 8, ValueType::Integer, &mut input_errors);
        let valley_width_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let valley_shape_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let bed_smoothing_converted = convert_input(inputs, 11, ValueType::Integer, &mut input_errors);
        let guide_strength_converted = convert_input(inputs, 12, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 13, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 14, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Image { data: guide_data, change_id: _ } = guide_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::Decimal(carve_depth) = carve_depth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(depth_exponent) = depth_exp_converted.unwrap() else { unreachable!() };
        let Value::Integer(river_width) = river_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(valley_width) = valley_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(valley_shape) = valley_shape_converted.unwrap() else { unreachable!() };
        let Value::Integer(bed_smoothing) = bed_smoothing_converted.unwrap() else { unreachable!() };
        let Value::Decimal(guide_strength) = guide_strength_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let amount = (amount as f64).clamp(0.0, 1.0);
        let carve_depth = (carve_depth as f64).clamp(0.0, 1.0);
        let depth_exponent = (depth_exponent as f64).clamp(0.1, 1.0);
        // River width is authored in reference pixels (at 1024px) and scaled to
        // the generator's own width/height so river size is the same relative
        // proportion at any output resolution.
        let river_width_px = scale_to_resolution(river_width.clamp(1, 64) as f32, width as u32, height as u32).round().max(1.0) as f64;
        // Valley width is a multiplier of the already-scaled channel width, so
        // it is NOT scaled again.
        let valley_width = (valley_width as f64).clamp(1.0, 32.0);
        let valley_shape = (valley_shape as f64).clamp(0.0, 1.0);
        let bed_smoothing = bed_smoothing.clamp(0, 10) as usize;
        let guide_strength = (guide_strength as f64).clamp(0.0, 1.0);
        let octaves = octaves.clamp(1, 16) as usize;
        let frequency = (frequency as f64).max(0.01);

        let w = width as usize;
        let h = height as usize;
        let n = w * h;

        // 1. Starting terrain: connected guidance map or seeded fBm fallback,
        // both in [0, 1]. This is the surface the final valleys carve into.
        let orig: Vec<f64> = if super::is_unconnected(&map_data) {
            super::fallback_terrain(seed as u32, w, h, octaves, frequency)
        } else {
            super::guidance_map_to_grid(&map_data, w, h)
        };

        // 2. Pilot carve (routing only): if a river guide is connected, dig a
        // shallow trench along bright guide pixels so flow prefers that path.
        // CRITICAL: the final carve in step 9 operates on `orig`, never on
        // `routed` — the pilot trench only steers routing and must not appear in
        // the output by itself. When unconnected, `routed` just borrows `orig`.
        let guide_connected = !super::is_unconnected(&guide_data);
        let routed_owned: Option<Vec<f64>> = if guide_connected {
            let guide = super::guidance_map_to_grid(&guide_data, w, h);
            Some(orig.iter().zip(guide.iter())
                .map(|(&o, &g)| o - PILOT_CARVE_FACTOR * guide_strength * g)
                .collect())
        } else {
            None
        };
        let routed: &[f64] = routed_owned.as_deref().unwrap_or(&orig);

        // 3. Priority-flood depression filling with epsilon drainage
        // (Barnes, Lehman & Mutz 2014). Seed the frontier with every border
        // cell (the ocean); pop the lowest, and lift each unvisited neighbor to
        // just above the current fill level so a strictly downhill path always
        // exists. Sequential, O(n log n), fully deterministic via the push
        // counter tie-break.
        let mut filled: Vec<f64> = routed.to_vec();
        {
            let mut visited = vec![false; n];
            let mut heap: BinaryHeap<HeapCell> = BinaryHeap::new();
            let mut push_counter: u64 = 0;
            for y in 0..h {
                for x in 0..w {
                    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                        let i = y * w + x;
                        visited[i] = true;
                        heap.push(HeapCell { h: filled[i], order: push_counter, idx: i as u32 });
                        push_counter += 1;
                    }
                }
            }
            while let Some(cell) = heap.pop() {
                let cur = cell.idx as usize;
                let cx = (cur % w) as i64;
                let cy = (cur / w) as i64;
                for &(dx, dy) in &NEIGH8 {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 { continue; }
                    let nb = ny as usize * w + nx as usize;
                    if visited[nb] { continue; }
                    visited[nb] = true;
                    filled[nb] = routed[nb].max(cell.h + FILL_EPS);
                    heap.push(HeapCell { h: filled[nb], order: push_counter, idx: nb as u32 });
                    push_counter += 1;
                }
            }
        }

        // 4. D8 flow routing (O'Callaghan & Marks 1984): each interior cell
        // drains to the neighbor of maximum descent slope; border cells drain
        // off the image (-1). Pure function of `filled`, so rayon-parallel.
        let recv: Vec<i32> = {
            let filled_ref = &filled;
            (0..n).into_par_iter().map(move |c| {
                let x = c % w;
                let y = c / w;
                if x == 0 || y == 0 || x == w - 1 || y == h - 1 { return -1; }
                let hc = filled_ref[c];
                let mut best_slope = 0.0_f64;
                let mut best = -1_i32;
                for &(dx, dy) in &NEIGH8 {
                    let nb = ((y as i64 + dy) * w as i64 + (x as i64 + dx)) as usize;
                    let hn = filled_ref[nb];
                    if hn < hc {
                        let dist = if dx.abs() + dy.abs() == 2 { std::f64::consts::SQRT_2 } else { 1.0 };
                        let slope = (hc - hn) / dist;
                        if slope > best_slope {
                            best_slope = slope;
                            best = nb as i32;
                        }
                    }
                }
                best
            }).collect()
        };

        // 5. Flow accumulation: process cells from highest filled height to
        // lowest, pushing each cell's accumulated area down to its receiver.
        // Epsilon fill guarantees receivers are strictly lower, so every donor
        // is processed before its receiver.
        let order: Vec<u32> = {
            let filled_ref = &filled;
            let mut o: Vec<u32> = (0..n as u32).collect();
            o.par_sort_unstable_by(|&a, &b| {
                filled_ref[b as usize].total_cmp(&filled_ref[a as usize]).then(a.cmp(&b))
            });
            o
        };
        let mut acc = vec![1.0_f64; n];
        for &c in &order {
            let r = recv[c as usize];
            if r >= 0 {
                acc[r as usize] += acc[c as usize];
            }
        }

        // 6. River extraction: cells whose drainage exceeds a resolution-
        // independent fraction of the total area become channels.
        let a_max = acc.iter().cloned().fold(1.0_f64, f64::max);
        let a_th = 16.0_f64.max(n as f64 * 10f64.powf(-1.0 - 3.0 * amount));
        let in_channel: Vec<bool> = acc.iter().map(|&a| a >= a_th).collect();
        let any_channel = in_channel.iter().any(|&b| b);

        // 7. Per-channel bed depth and width. Depth grows with drainage area
        // (stream power); width grows with sqrt(drainage) (hydraulic geometry,
        // Leopold & Maddock 1953). bed = filled - depth is automatically
        // monotone downstream (filled falls, depth rises along flow).
        let mut width_of = vec![0.0_f64; n];
        let mut bed = vec![0.0_f64; n];
        if any_channel {
            for c in 0..n {
                if !in_channel[c] { continue; }
                let ratio = acc[c] / a_max;
                let d = carve_depth * ratio.powf(depth_exponent);
                width_of[c] = (river_width_px * ratio.sqrt()).max(MIN_CHANNEL_WIDTH);
                bed[c] = filled[c] - d;
            }

            // For each channel cell, its steepest in-channel donor (the in-
            // channel cell draining into it with the most accumulated area).
            let mut best_donor = vec![-1_i32; n];
            let mut best_donor_acc = vec![f64::NEG_INFINITY; n];
            for c in 0..n {
                if !in_channel[c] { continue; }
                let r = recv[c];
                if r >= 0 && in_channel[r as usize] {
                    let r = r as usize;
                    if acc[c] > best_donor_acc[r] {
                        best_donor_acc[r] = acc[c];
                        best_donor[r] = c as i32;
                    }
                }
            }

            // Bed smoothing: along-flow 3-tap average over {self, receiver,
            // steepest donor} — all in-channel. Relaxes the bed without
            // wandering off the channel.
            let mut buf = bed.clone();
            for _ in 0..bed_smoothing {
                for c in 0..n {
                    if !in_channel[c] { buf[c] = bed[c]; continue; }
                    let mut sum = bed[c];
                    let mut cnt = 1.0_f64;
                    let r = recv[c];
                    if r >= 0 && in_channel[r as usize] { sum += bed[r as usize]; cnt += 1.0; }
                    let dn = best_donor[c];
                    if dn >= 0 { sum += bed[dn as usize]; cnt += 1.0; }
                    buf[c] = sum / cnt;
                }
                std::mem::swap(&mut bed, &mut buf);
            }
            drop(buf);
            drop(best_donor);
            drop(best_donor_acc);

            // One re-enforcement sweep, highest to lowest, restoring strict
            // downstream monotonicity that averaging may have broken.
            for &c in &order {
                let c = c as usize;
                if !in_channel[c] { continue; }
                let r = recv[c];
                if r >= 0 && in_channel[r as usize] {
                    let r = r as usize;
                    bed[r] = bed[r].min(bed[c] - FILL_EPS);
                }
            }
        }

        // 8. Labeled distance transform: squared distance to the nearest channel
        // cell plus that cell's flat index.
        let (d2, label) = super::distance_field_labeled(&in_channel, w, h);

        // 9. Carve pass: within a channel's width, drop terrain to the flat bed;
        // out to the valley width, blend the wall from bed back up to the
        // original surface with a V-to-U profile; beyond that, leave it. The
        // outer `min` never RAISES terrain. Rayon per-pixel.
        let carved: Vec<f64> = if any_channel {
            let orig_ref = &orig;
            let bed_ref = &bed;
            let width_ref = &width_of;
            let d2_ref = &d2;
            let label_ref = &label;
            (0..n).into_par_iter().map(move |p| {
                let lab = label_ref[p];
                if lab == u32::MAX { return orig_ref[p]; }
                let c = lab as usize;
                let dst = d2_ref[p].sqrt();
                let wc = width_ref[c];
                let wv = wc * valley_width;
                if dst <= wc {
                    orig_ref[p].min(bed_ref[c])
                } else if dst <= wv {
                    let s = (dst - wc) / (wv - wc);
                    let q = 1.0 + 2.0 * valley_shape;
                    orig_ref[p].min(bed_ref[c] + (orig_ref[p] - bed_ref[c]) * s.powf(q))
                } else {
                    orig_ref[p]
                }
            }).collect()
        } else {
            orig.clone()
        };

        // 10. Water depth: rivers fill to the pre-carve fill level of their
        // governing channel cell, PLUS lakes everywhere (the fill delta the
        // priority-flood added over the routing terrain). Zero when no channels.
        let depth: Vec<f64> = if any_channel {
            let carved_ref = &carved;
            let filled_ref = &filled;
            let width_ref = &width_of;
            let d2_ref = &d2;
            let label_ref = &label;
            (0..n).into_par_iter().map(move |p| {
                let mut dp = 0.0_f64;
                let lab = label_ref[p];
                if lab != u32::MAX {
                    let c = lab as usize;
                    let dst = d2_ref[p].sqrt();
                    if dst <= width_ref[c] {
                        dp = (filled_ref[c] - carved_ref[p]).max(0.0);
                    }
                }
                dp += (filled_ref[p] - routed[p]).max(0.0);
                dp
            }).collect()
        } else {
            vec![0.0; n]
        };

        // 11. Build the four output grids.

        // Height: normalize the carved terrain to [0, 1].
        let min_h = carved.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_h = carved.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = (max_h - min_h).max(1e-10);
        let height_grid: Vec<f64> = carved.iter().map(|&v| (v - min_h) / range).collect();

        // River mask: log-scaled by the governing channel cell's drainage,
        // painted across that channel's width.
        let river_mask: Vec<f64> = if any_channel {
            let denom = (a_max / a_th).ln() + 1.0;
            let acc_ref = &acc;
            let width_ref = &width_of;
            let d2_ref = &d2;
            let label_ref = &label;
            (0..n).into_par_iter().map(move |p| {
                let lab = label_ref[p];
                if lab == u32::MAX { return 0.0; }
                let c = lab as usize;
                let dst = d2_ref[p].sqrt();
                if dst <= width_ref[c] {
                    (((acc_ref[c] / a_th).ln() + 1.0) / denom).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }).collect()
        } else {
            vec![0.0; n]
        };

        // Flow map: log-scaled drainage at EVERY pixel (whole tributary tree).
        let ln_amax1 = (1.0 + a_max).ln();
        let flow_grid: Vec<f64> = acc.iter().map(|&a| (1.0 + a).ln() / ln_amax1).collect();

        // Water depth: normalize by its own max.
        let dmax = depth.iter().cloned().fold(0.0_f64, f64::max);
        let depth_grid: Vec<f64> = if dmax > 0.0 {
            depth.iter().map(|&v| v / dmax).collect()
        } else {
            vec![0.0; n]
        };

        // Free the big f64 intermediates before allocating the output images.
        drop(filled);
        drop(acc);
        drop(order);
        drop(d2);
        drop(label);
        drop(carved);
        drop(depth);

        let height_image = grid_to_image(&height_grid, w, h);
        let mask_image = grid_to_image(&river_mask, w, h);
        let flow_image = grid_to_image(&flow_grid, w, h);
        let depth_image = grid_to_image(&depth_grid, w, h);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(mask_image), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(flow_image), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(depth_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "rivers_tests.rs"]
mod tests;
