//! Carve-river node.
//!
//! Conforms a terrain to a hand-authored river path mask so the river sits in
//! a real valley with a bed that never runs uphill — the fix for the "river
//! slapped onto terrain" look. This is a heuristic terrain-conditioning tool,
//! not a flow simulation.
//!
//! The mask is thresholded to a set of "on" pixels, widened into a real
//! channel via an exact Euclidean distance transform (Felzenszwalb &
//! Huttenlocher), and a valley profile is carved that falls off from the
//! channel bed back up to the untouched terrain. Terrain is only ever lowered,
//! never raised.
//!
//! The monotonic-bed step is the core: channel pixels are ordered by
//! along-channel distance from their outlets (image-border touches, or each
//! disconnected blob's lowest point) and a water line is propagated downstream
//! so the bed never runs uphill. Where the user's path crosses a ridge, a
//! gorge is carved through it instead of the water climbing the hill.
//!
//! Everything is deterministic (no RNG except the seed handed to the fallback
//! terrain), and rayon is used only for order-independent per-pixel passes.
//! Unlike hydraulic erosion, the output does NOT tile: water drains at the
//! image edges.

use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::tone_curve::{sample_lut, tone_curve_lut, TONE_LUT_SIZE};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

/// Downhill nudge applied each min-propagation step so the water line is
/// strictly non-increasing along the channel toward every outlet.
const EPS: f64 = 1e-7;

/// Default valley-profile tone curve: a 5-point Smooth approximation of the
/// previous scalar default (`valley shape` 0.5 → exponent q = 2, i.e. f(s) =
/// s²). Decoded (output = 1 − y) the control points hit exactly 0, 0.0625,
/// 0.25, 0.5625, 1 at s = 0, 0.25, 0.5, 0.75, 1 — the s² values.
fn default_valley_profile() -> Curve {
    Curve {
        points: vec![[0.0, 1.0], [0.25, 0.9375], [0.5, 0.75], [0.75, 0.4375], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    }
}

/// Encodes a normalized linear value to non-linear sRGB, matching the encoding
/// hydraulic erosion uses for its height output.
#[inline]
fn encode(value: f32) -> f32 {
    crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(value)
}

/// Builds the two output images from the final carved terrain and (optional)
/// water-depth grid. The carved grid is min/max normalized to [0, 1]; the
/// depth grid is normalized by its own maximum (all black when there is no
/// water or no depth grid at all). Both are sRGB-encoded per pixel.
fn build_images(carved: &[f64], depth: Option<&[f64]>, w: usize, h: usize) -> (FloatImage, FloatImage) {
    // Height: min/max normalize then sRGB encode.
    let min_h = carved.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_h = carved.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max_h - min_h).max(1e-10);
    let mut height_image = FloatImage::new(w as u32, h as u32, 1);
    for y in 0..h {
        for x in 0..w {
            let normalized = ((carved[y * w + x] - min_h) / range) as f32;
            height_image.put_pixel(x as u32, y as u32, &[encode(normalized)]);
        }
    }

    // Water depth: normalize by its own max; guard against an all-zero grid so
    // the output is simply black when there is no water.
    let mut depth_image = FloatImage::new(w as u32, h as u32, 1);
    if let Some(d) = depth {
        let dmax = d.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if dmax > 0.0 {
            for y in 0..h {
                for x in 0..w {
                    let normalized = (d[y * w + x] / dmax) as f32;
                    depth_image.put_pixel(x as u32, y as u32, &[encode(normalized)]);
                }
            }
        }
    }

    (height_image, depth_image)
}

/// One 3x3 box-blur pass over `src` into `dst`, clamping at the image edges.
/// Rayon-parallel per row; order-independent.
fn box_blur_pass(src: &[f64], dst: &mut [f64], w: usize, h: usize) {
    dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let mut sum = 0.0;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let nx = (x as i64 + dx).clamp(0, w as i64 - 1) as usize;
                    let ny = (y as i64 + dy).clamp(0, h as i64 - 1) as usize;
                    sum += src[ny * w + nx];
                }
            }
            row[x] = sum / 9.0;
        }
    });
}

/// Operation that conforms a terrain to a user-authored river path mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageSimulationCarveRiver {}

impl OpImageSimulationCarveRiver {
    /// Returns the node metadata (name, description, help) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "carve river".to_string(),
            description: "Conforms terrain to a hand-authored river path mask so water sits in a real valley instead of looking pasted on: distance-transform valley carve with a monotonic (never-uphill) bed.".to_string(),
            help: "Conforms terrain to a hand-authored river path so water sits in a real valley instead of looking pasted on. A heuristic terrain-conditioning tool, not a flow simulation: the mask is thresholded, widened via an exact Euclidean distance transform (Felzenszwalb & Huttenlocher), and a valley profile carved that falls off from the bed to untouched terrain - terrain is only ever lowered, never raised.\n\nThe monotonic bed step orders channel pixels by along-channel distance from the outlets (border touches, or each disconnected blob's lowest point) and propagates the water line downstream so the bed never runs uphill - where the path crosses a ridge, a gorge is carved through it; untick monotonic bed to see the naive slapped-on carve. The height map is optional (fBm terrain from the seed when unconnected; octaves and frequency shape it, and are ignored otherwise). The river mask can come from the line, lightning, or veins nodes or any painted image.\n\nRiver width widens thin paths; valley width sets how far the walls reach and valley profile is a drawn curve for the valley cross-section (x = distance from the channel, 0 = bank; y = how much of the original terrain height remains, bottom = river bed - the default approximates a smooth V, hug the bottom longer for a flat-floored U); bank smoothing softens the carve edges. Widths are authored at a 1024px reference and scale with the output size. Water drains at the image edges and the output does NOT tile (unlike hydraulic erosion). Deterministic. Outputs the carved height and a water-depth map.".to_string(),
        }
    }

    /// Creates the default inputs in the simulation convention: seed and
    /// dimensions first, then the optional height/mask guidance maps, then the
    /// carve/valley drivers, and the fallback-terrain params last.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the fallback terrain (only used when no height map is connected)."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("height map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional starting terrain to carve; when unconnected an internal fBm heightmap is generated from the seed."),
            Input::new("river mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional river-path mask (bright = river). Can come from the line, lightning, or veins nodes or any painted image; when unconnected the terrain passes through unchanged."),
            Input::new("mask threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Brightness above which a mask pixel counts as river."),
            Input::new("carve depth".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How far the channel bed is lowered below the water line, in normalized height units."),
            Input::new("river width".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((0.0, 128.0)), speed: None }), None)
                .with_description("Widens thin mask paths into a channel, in pixels at a 1024px reference (scales with output size); 0 carves only the exact mask pixels."),
            Input::new("valley width".to_string(), Value::Integer(48), Some(InputSettings::DragValue { clamp: Some((0.0, 512.0)), speed: None }), None)
                .with_description("Width of the valley walls falling off from the channel to untouched terrain, in pixels at a 1024px reference (scales with output size)."),
            Input::new("valley profile".to_string(), Value::Curve(default_valley_profile()), Some(InputSettings::ToneCurve), None)
                .with_description("Valley cross-section: x = distance from the channel (0 = bank, 1 = valley rim), y = how much of the original terrain height remains (bottom = river bed). The default approximates a smooth V; hug the bottom edge longer for a wide flat-floored U."),
            Input::new("bank smoothing".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of box-blur passes softening the carve edges so the banks are not razor-sharp."),
            Input::new("monotonic bed".to_string(), Value::Bool(true), None, None)
                .with_description("Force the channel bed to never run uphill (carving a gorge where the path crosses a ridge). Untick to see the naive slapped-on carve."),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of fBm octaves in the fallback terrain; only used when no height map is connected."),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Base frequency of the fallback terrain; only used when no height map is connected."),
        ]
    }

    /// Creates the two outputs: the carved heightmap and the water-depth map.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Carved grayscale heightmap, normalized to the 0-1 range."),
            Output::new("water depth".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Water depth in the channel (distance from the carved bed up to the enforced water line), normalized to its own maximum; black outside the channel."),
        ]
    }

    /// Runs the river-carving pass.
    ///
    /// 1. Builds the starting terrain (connected map or seeded fBm) and the
    ///    thresholded river mask; passthrough when nothing is on
    /// 2. Widens the mask into a channel region C via a distance transform
    /// 3. Computes the water line along C (monotonic downstream when enabled)
    /// 4. Carves a valley profile falling off from the bed to untouched terrain
    /// 5. Smooths the banks and normalizes the carved height + water depth
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let map_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let mask_converted = convert_input(inputs, 4, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let carve_depth_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let river_width_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);
        let valley_width_converted = convert_input(inputs, 8, ValueType::Integer, &mut input_errors);
        let valley_profile_converted = convert_input(inputs, 9, ValueType::Curve, &mut input_errors);
        let bank_smoothing_converted = convert_input(inputs, 10, ValueType::Integer, &mut input_errors);
        let monotonic_converted = convert_input(inputs, 11, ValueType::Bool, &mut input_errors);
        let octaves_converted = convert_input(inputs, 12, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 13, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Image { data: mask_data, change_id: _ } = mask_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(carve_depth) = carve_depth_converted.unwrap() else { unreachable!() };
        let Value::Integer(river_width) = river_width_converted.unwrap() else { unreachable!() };
        let Value::Integer(valley_width) = valley_width_converted.unwrap() else { unreachable!() };
        let Value::Curve(valley_profile) = valley_profile_converted.unwrap() else { unreachable!() };
        let Value::Integer(bank_smoothing) = bank_smoothing_converted.unwrap() else { unreachable!() };
        let Value::Bool(monotonic) = monotonic_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        // Clamp all parameters to their documented ranges.
        width = width.clamp(1, 4096);
        height = height.clamp(1, 4096);
        seed = seed.max(1);
        let octaves = octaves.clamp(1, 16) as usize;
        let frequency = (frequency as f64).max(0.01);
        let threshold = (threshold as f64).clamp(0.0, 1.0);
        let carve_depth = (carve_depth as f64).clamp(0.0, 1.0);
        let bank_smoothing = bank_smoothing.clamp(0, 10) as usize;
        // Widths are authored at a 1024px reference and scaled to the actual
        // output size, so the same value produces the same relative channel /
        // valley width at any resolution. 0 stays 0 (carve exact mask pixels).
        let river_width_px = scale_to_resolution(river_width.clamp(0, 128) as f32, width as u32, height as u32).round() as f64;
        let valley_width_px = scale_to_resolution(valley_width.clamp(0, 512) as f32, width as u32, height as u32).round() as f64;

        let w = width as usize;
        let h = height as usize;
        let n = w * h;

        // 1. Starting terrain: connected guidance map or seeded fBm fallback.
        let orig: Vec<f64> = if super::is_unconnected(&map_data) {
            super::fallback_terrain(seed as u32, w, h, octaves, frequency)
        } else {
            super::guidance_map_to_grid(&map_data, w, h)
        };

        // With no mask connected the terrain passes through unchanged and the
        // water depth is all black.
        if super::is_unconnected(&mask_data) {
            let (height_image, depth_image) = build_images(&orig, None, w, h);
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
                    OutputResponse { value: Value::Image { data: Arc::new(depth_image), change_id: get_id() } },
                ],
            });
        }

        // 2. Threshold the mask. No river pixels ⇒ same passthrough.
        let mask = super::guidance_map_to_grid(&mask_data, w, h);
        let on: Vec<bool> = mask.iter().map(|&m| m >= threshold).collect();
        if !on.iter().any(|&b| b) {
            let (height_image, depth_image) = build_images(&orig, None, w, h);
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
                    OutputResponse { value: Value::Image { data: Arc::new(depth_image), change_id: get_id() } },
                ],
            });
        }

        // 3. Channel region C: widen the thresholded path by river_width via an
        // exact distance transform, so 1px paths become a real channel. When
        // river_width_px is 0 this reduces to the on-pixels themselves.
        let (d2_mask, _) = super::distance_field_labeled(&on, w, h);
        let rwpx2 = river_width_px * river_width_px;
        let in_c: Vec<bool> = (0..n).map(|p| d2_mask[p] <= rwpx2).collect();

        // 4. Second labeled distance transform from C: the squared distance
        // drives the valley falloff and the label gives each pixel its nearest
        // (governing) channel cell.
        let (d2c, label) = super::distance_field_labeled(&in_c, w, h);

        // 5. Water line W along C. Naive mode leaves W = orig (a slapped-on
        // carve that follows the terrain). Monotonic mode propagates a
        // never-uphill water line downstream from the outlets.
        let mut water_line = orig.clone();
        if monotonic {
            // Outlets: every C cell on the image border, plus each border-less
            // 8-connected component's lowest (min-orig) cell so isolated blobs
            // still drain somewhere.
            let mut outlet = vec![false; n];
            let mut visited = vec![false; n];
            for start in 0..n {
                if !in_c[start] || visited[start] { continue; }
                // Flood the component (DFS; order only affects traversal, not
                // the resulting outlet set).
                let mut stack = vec![start];
                visited[start] = true;
                let mut component = Vec::new();
                let mut has_border = false;
                while let Some(cur) = stack.pop() {
                    component.push(cur);
                    let cx = cur % w;
                    let cy = cur / w;
                    if cx == 0 || cy == 0 || cx == w - 1 || cy == h - 1 {
                        has_border = true;
                        outlet[cur] = true;
                    }
                    for dy in -1i64..=1 {
                        for dx in -1i64..=1 {
                            if dx == 0 && dy == 0 { continue; }
                            let nx = cx as i64 + dx;
                            let ny = cy as i64 + dy;
                            if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 { continue; }
                            let np = ny as usize * w + nx as usize;
                            if in_c[np] && !visited[np] {
                                visited[np] = true;
                                stack.push(np);
                            }
                        }
                    }
                }
                if !has_border {
                    // Lowest cell of the blob; tie-break on the lowest index.
                    let mut best = component[0];
                    for &idx in &component {
                        if orig[idx] < orig[best] || (orig[idx] == orig[best] && idx < best) {
                            best = idx;
                        }
                    }
                    outlet[best] = true;
                }
            }

            // gdist: multi-source BFS across C from all outlets (8-connectivity)
            // — the along-channel distance to the nearest outlet.
            let mut gdist = vec![u32::MAX; n];
            let mut queue: VecDeque<usize> = VecDeque::new();
            for p in 0..n {
                if outlet[p] {
                    gdist[p] = 0;
                    queue.push_back(p);
                }
            }
            while let Some(cur) = queue.pop_front() {
                let d = gdist[cur];
                let cx = cur % w;
                let cy = cur / w;
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = cx as i64 + dx;
                        let ny = cy as i64 + dy;
                        if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 { continue; }
                        let np = ny as usize * w + nx as usize;
                        if in_c[np] && gdist[np] == u32::MAX {
                            gdist[np] = d + 1;
                            queue.push_back(np);
                        }
                    }
                }
            }

            // Bucket C cells by gdist for an O(n) decreasing-distance sweep.
            let max_gdist = (0..n)
                .filter(|&p| in_c[p] && gdist[p] != u32::MAX)
                .map(|p| gdist[p])
                .max()
                .unwrap_or(0);
            let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); max_gdist as usize + 1];
            for p in 0..n {
                if in_c[p] && gdist[p] != u32::MAX {
                    buckets[gdist[p] as usize].push(p as u32);
                }
            }

            // Min-propagation: visit cells in decreasing gdist and push the
            // water line down to EVERY C-neighbour strictly closer to an
            // outlet. Because higher-gdist cells are processed first, W is
            // non-increasing along the channel toward every outlet — where the
            // path climbs a ridge, W stays at the upstream level and the ridge
            // later gets a gorge. Propagating to all downstream neighbours
            // (rather than one chosen receiver) makes W uniform across a
            // widened channel's cross-section, so the whole bed drops together
            // instead of a single deep thread; min() is commutative, so the
            // result is deterministic with no tie-breaking needed.
            let enforce = |wl: &mut [f64]| {
                for g in (1..=max_gdist as usize).rev() {
                    for &cell_u in &buckets[g] {
                        let cell = cell_u as usize;
                        let cx = cell % w;
                        let cy = cell / w;
                        let pushed = wl[cell] - EPS;
                        for dy in -1i64..=1 {
                            for dx in -1i64..=1 {
                                if dx == 0 && dy == 0 { continue; }
                                let nx = cx as i64 + dx;
                                let ny = cy as i64 + dy;
                                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 { continue; }
                                let np = ny as usize * w + nx as usize;
                                if !in_c[np] || gdist[np] >= g as u32 { continue; }
                                wl[np] = wl[np].min(pushed);
                            }
                        }
                    }
                }
            };
            enforce(&mut water_line);

            // The running-min water line is a staircase: long plateaus with
            // abrupt drops wherever the terrain dips, which carve as blocky
            // steps along the channel. Smooth W along the channel (a 3x3 mean
            // restricted to C, enough passes to spread a drop over roughly the
            // valley width), then re-run the enforcement sweep so downstream
            // monotonicity stays exact and only the ramps ease in.
            let smooth_passes = ((river_width_px + valley_width_px) as usize).clamp(4, 64);
            let mut scratch = water_line.clone();
            for _ in 0..smooth_passes {
                for bucket in &buckets {
                    for &cell_u in bucket {
                        let cell = cell_u as usize;
                        let cx = cell % w;
                        let cy = cell / w;
                        let mut sum = water_line[cell];
                        let mut count = 1.0;
                        for dy in -1i64..=1 {
                            for dx in -1i64..=1 {
                                if dx == 0 && dy == 0 { continue; }
                                let nx = cx as i64 + dx;
                                let ny = cy as i64 + dy;
                                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 { continue; }
                                let np = ny as usize * w + nx as usize;
                                if !in_c[np] { continue; }
                                sum += water_line[np];
                                count += 1.0;
                            }
                        }
                        scratch[cell] = sum / count;
                    }
                }
                std::mem::swap(&mut water_line, &mut scratch);
            }
            enforce(&mut water_line);
        }

        // 6. Bed: the water line lowered by the carve depth, defined on C.
        let mut bed = vec![0.0_f64; n];
        for p in 0..n {
            if in_c[p] {
                bed[p] = water_line[p] - carve_depth;
            }
        }

        // 7. Carve pass (rayon per pixel, order-independent). In-channel cells
        // drop to their own bed; valley cells interpolate from the nearest
        // channel cell's bed up to the untouched terrain along the drawn
        // valley-profile curve f(s) (s = normalized distance from the
        // channel, f = fraction of the original height remaining); everything
        // else is left alone. LUT endpoint bins are exact, so the default
        // profile keeps f(0) = 0 (bed at the bank) and f(1) = 1 (untouched at
        // the rim); sample_lut clamps its output to [0, 1], so any user curve
        // keeps the carve between the bed and the original terrain, and the
        // outer min() with orig guarantees terrain is only ever lowered,
        // never raised.
        let profile_lut = tone_curve_lut(&valley_profile, TONE_LUT_SIZE);
        let profile_lut_ref = &profile_lut;
        let orig_ref = &orig;
        let bed_ref = &bed;
        let label_ref = &label;
        let d2c_ref = &d2c;
        let in_c_ref = &in_c;
        let mut carved: Vec<f64> = (0..n).into_par_iter().map(|p| {
            if in_c_ref[p] {
                orig_ref[p].min(bed_ref[p])
            } else {
                let dst = d2c_ref[p].sqrt();
                if valley_width_px > 0.0 && dst <= valley_width_px {
                    let c = label_ref[p] as usize;
                    let bed_c = bed_ref[c];
                    let s = dst / valley_width_px;
                    let f = sample_lut(profile_lut_ref, s as f32) as f64;
                    orig_ref[p].min(bed_c + (orig_ref[p] - bed_c) * f)
                } else {
                    orig_ref[p]
                }
            }
        }).collect();

        // 8. Bank smoothing: blur the carve delta (orig - carved, >= 0
        // everywhere) so the banks are not razor-sharp, then re-derive carved.
        // Re-clamp C cells to their bed afterwards so smoothing can never lift
        // the bed above the enforced water line.
        if bank_smoothing > 0 {
            let mut delta: Vec<f64> = (0..n).map(|p| orig[p] - carved[p]).collect();
            let mut scratch = vec![0.0_f64; n];
            for _ in 0..bank_smoothing {
                box_blur_pass(&delta, &mut scratch, w, h);
                std::mem::swap(&mut delta, &mut scratch);
            }
            for p in 0..n {
                carved[p] = orig[p] - delta[p];
                if in_c[p] {
                    carved[p] = carved[p].min(bed[p]);
                }
            }
        }

        // 9. Water depth: fill each channel cell from its carved bed up to the
        // enforced water line; zero everywhere else.
        let depth: Vec<f64> = (0..n).map(|p| {
            if in_c[p] {
                (water_line[p] - carved[p]).max(0.0)
            } else {
                0.0
            }
        }).collect();

        // 10. Normalize and encode both outputs.
        let (height_image, depth_image) = build_images(&carved, Some(&depth), w, h);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(depth_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "carve_river_tests.rs"]
mod tests;
