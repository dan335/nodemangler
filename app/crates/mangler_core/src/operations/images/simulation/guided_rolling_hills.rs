//! Guided rolling hills: `rolling hills` shaped around a river.
//!
//! Splats the same Hann-kernel hills as the `rolling hills` noise node, then
//! (when a guidance image is connected) suppresses them near a channel and
//! carves a flat bed with rising banks, so the result reads as hills around a
//! river instead of hills with a river slapped on top. The guidance image is
//! either a river mask (thresholded, then widened/falloff-shaped via an exact
//! distance transform, the `carve_river` approach) or an already-computed
//! distance field used directly as the falloff driver (e.g. the adjustments
//! `distance` node, so the user can shape the profile themselves upstream).
//!
//! Heuristic composition, not a physical model. Unconnected (or an
//! all-zero/all-below-threshold mask) falls back to plain rolling hills and
//! tiles; once a guidance map drives the distance field the output does NOT
//! tile (the EDT is not toroidal), matching `carve_river`.

use rayon::prelude::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::noise::voronoi_common::{cell_hash, wrap_cell};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Encodes a normalized linear value to non-linear sRGB, matching rolling
/// hills / carve river's output encoding.
#[inline]
fn encode(value: f32) -> f32 {
    crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(value)
}

/// Splats one Hann-kernel hill per jittered grid cell and min/max normalizes
/// to [0, 1]. Verbatim port of `noise::cellular::rolling_hills`'s splat loop
/// and normalization (`rolling_hills.rs:118-194`) so an unconnected guidance
/// map reproduces that node's output pixel-for-pixel; if `rolling_hills.rs`
/// changes, update this copy to match.
fn splat_hills_normalized(
    seed: u32,
    width: i32,
    height: i32,
    density: f64,
    size: f64,
    size_variation: f64,
    height_variation: f64,
    peakiness: f64,
    merge: f64,
) -> Vec<f64> {
    let grid = density.round().max(1.0) as i32;
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
                    let wx = wrap_cell(cell_x + dx, grid);
                    let wy = wrap_cell(cell_y + dy, grid);

                    let kx = (cell_x + dx) as f64 + cell_hash(wx, wy, seed, 0);
                    let ky = (cell_y + dy) as f64 + cell_hash(wx, wy, seed, 1);

                    let r = size * (1.0 - size_variation + 2.0 * size_variation * cell_hash(wx, wy, seed, 2));

                    let ddx = gx - kx;
                    let ddy = gy - ky;
                    let d2 = ddx * ddx + ddy * ddy;
                    if d2 >= r * r {
                        continue;
                    }

                    let t = d2.sqrt() / r;
                    let amp = 1.0 - height_variation * cell_hash(wx, wy, seed, 3);

                    let contribution = amp * (0.5 + 0.5 * (std::f64::consts::PI * t).cos()).powf(peakiness);
                    sum += contribution;
                    tallest = tallest.max(contribution);
                }
            }

            tallest + merge * (sum - tallest)
        })
    }).collect();

    let min_val = buffer.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = buffer.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;

    buffer.iter().map(|&v| {
        if range < 1e-12 { 0.5 } else { (v - min_val) / range }
    }).collect()
}

/// Builds the plain-hills fallback response used both when the guidance map
/// is unconnected and when a connected mask has no on-pixels: the height
/// output is `hills01` encoded (pixel-identical to plain rolling hills), and
/// the channel mask is all black.
fn fallback_response(hills01: &[f64], w: usize, h: usize, start_time: Instant) -> OperationResponse {
    let mut height_image = FloatImage::new(w as u32, h as u32, 1);
    for y in 0..h {
        for x in 0..w {
            height_image.put_pixel(x as u32, y as u32, &[encode(hills01[y * w + x] as f32)]);
        }
    }
    let mask_image = FloatImage::new(w as u32, h as u32, 1);

    OperationResponse {
        time: Instant::now().duration_since(start_time),
        responses: vec![
            OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
            OutputResponse { value: Value::Image { data: Arc::new(mask_image), change_id: get_id() } },
        ],
    }
}

/// Operation that generates rolling hills shaped around a river mask or
/// distance field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageSimulationGuidedRollingHills {}

impl OpImageSimulationGuidedRollingHills {
    /// Returns the node metadata (name, description, help) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "guided rolling hills".to_string(),
            description: "Rolling hills that fade to a flat channel bed with rising banks around a river mask or distance field, instead of hills with a river slapped on top.".to_string(),
            help: "Heuristic composition, not a physical model: the same Hann-splat hill scatter as the rolling hills node (see that node's help), modulated by a distance ramp so hills fade out near a channel and a flat bed with rising banks/levee takes over. Unconnected (or a connected mask with no pixels above threshold), the output is pixel-identical to plain rolling hills and tiles; once a guidance map drives the ramp the output does NOT tile (the distance transform is not toroidal, same as carve river).\n\nTwo guidance modes, switched by 'map is distance field':\n- Mask mode (default): the guidance image is a river mask, bright = river (e.g. the meander node's river mask). Pixels at or above mask threshold are widened into a channel by river width, then valley width sets how far the banks fall off back to full hill height, both authored as pixels at a 1024px reference and scaled to the output size.\n- Distance-field mode: the guidance image is already a falloff driver, bright = channel (the adjustments distance node's output plugs in directly with no inversion needed) - mask threshold, river width, and valley width are ignored, and the bed follows the field's own shape rather than being perfectly flat; flatten it with a levels node upstream if you want a flat floor.\n\nValley shape blends V-profile (0) to flat-floored U (1) valley walls, in both modes. River depth is how much of the relief the valley ramp claims versus the hills; bank height adds a levee bump peaking partway up the bank (0 for no levee). Density/size/size variation/height variation/peakiness/merge are the same hill-shape controls as rolling hills. Deterministic from seed.\n\nOutputs the composed heightmap plus a channel mask (bright in the channel, fading to black at the valley rim, all black when nothing is connected) - a ready-made wetness/water mask aligned with the carve.".to_string(),
        }
    }

    /// Creates the default inputs in the simulation convention: seed and
    /// dimensions, the optional guidance map and its mode/shape params, then
    /// the hill-shape params (copied verbatim from rolling hills) last.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for hill placement and sizes; change to rearrange the hills."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("guidance map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional river mask (bright = river, e.g. the meander node's river mask) or, with 'map is distance field' on, a falloff driver (bright = channel - the distance node's output plugs in directly). Unconnected behaves exactly like plain rolling hills."),
            Input::new("map is distance field".to_string(), Value::Bool(false), None, None)
                .with_description("Off: guidance map is a river mask, thresholded and widened into a channel. On: guidance map is already a distance-like falloff (bright = channel) used directly; mask threshold, river width, and valley width are ignored."),
            Input::new("mask threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Brightness above which a mask pixel counts as river. Mask mode only; ignored when 'map is distance field' is on."),
            Input::new("river width".to_string(), Value::Integer(8), Some(InputSettings::DragValue { clamp: Some((0.0, 128.0)), speed: None }), None)
                .with_description("Widens thin mask paths into a channel, in pixels at a 1024px reference (scales with output size). Mask mode only; ignored when 'map is distance field' is on."),
            Input::new("valley width".to_string(), Value::Integer(96), Some(InputSettings::DragValue { clamp: Some((1.0, 512.0)), speed: None }), None)
                .with_description("Width of the valley walls falling off from the channel back to full hill height, in pixels at a 1024px reference (scales with output size). Mask mode only; ignored when 'map is distance field' is on."),
            Input::new("valley shape".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Valley wall profile: 0 = straight V walls, 1 = wide flat-floored U. Applies in both modes."),
            Input::new("river depth".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Fraction of the relief claimed by the valley ramp versus the hills; higher sinks the channel deeper relative to the surrounding hills."),
            Input::new("bank height".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Height of a levee bump peaking partway up the bank; 0 for no levee."),
            Input::new("density".to_string(), Value::Decimal(6.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.1) }), None)
                .with_description("Hills per axis; snapped to an integer grid internally so the pattern tiles when unconnected."),
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
        ]
    }

    /// Creates the two outputs: the composed heightmap and the channel mask.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Composed grayscale heightmap in [0, 1], sRGB-encoded. Tiles only when no guidance map is connected."),
            Output::new("channel mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Bright in the channel, fading to black at the valley rim; all black when nothing is connected. A ready-made water/wetness mask aligned with the carve."),
        ]
    }

    /// Runs the guided rolling hills generation.
    ///
    /// 1. Splats hills and min/max normalizes to `hills01` (before any
    ///    compositing, so the channel bed stays exactly flat/at-field).
    /// 2. Unconnected map (or an on-pixel-free mask) -> plain hills fallback.
    /// 3. Builds a normalized distance `d` from the guidance map, either
    ///    directly (distance-field mode) or via a labeled EDT of the
    ///    thresholded mask (mask mode).
    /// 4. Composites the valley ramp, hill modulation, and levee bump into
    ///    the final height, plus the channel mask from the ramp alone.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let map_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let is_df_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);
        let threshold_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let river_width_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);
        let valley_width_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);
        let valley_shape_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let river_depth_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let bank_height_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let density_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);
        let size_converted = convert_input(inputs, 12, ValueType::Decimal, &mut input_errors);
        let size_var_converted = convert_input(inputs, 13, ValueType::Decimal, &mut input_errors);
        let height_var_converted = convert_input(inputs, 14, ValueType::Decimal, &mut input_errors);
        let peakiness_converted = convert_input(inputs, 15, ValueType::Decimal, &mut input_errors);
        let merge_converted = convert_input(inputs, 16, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Bool(is_distance_field) = is_df_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Integer(river_width) = river_width_converted.unwrap() else { unreachable!() };
        let Value::Integer(valley_width) = valley_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(valley_shape) = valley_shape_converted.unwrap() else { unreachable!() };
        let Value::Decimal(river_depth) = river_depth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(bank_height) = bank_height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(size) = size_converted.unwrap() else { unreachable!() };
        let Value::Decimal(size_variation) = size_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(height_variation) = height_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(peakiness) = peakiness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(merge) = merge_converted.unwrap() else { unreachable!() };

        // Width/height/seed/hill-shape clamps match rolling hills exactly
        // (including the lack of an upper width/height bound) so the
        // unconnected fallback is pixel-identical to that node.
        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let density = (density as f64).clamp(1.0, 32.0);
        let size = (size as f64).clamp(0.5, 2.5);
        let size_variation = (size_variation as f64).clamp(0.0, 1.0);
        let height_variation = (height_variation as f64).clamp(0.0, 1.0);
        let peakiness = (peakiness as f64).clamp(0.25, 4.0);
        let merge = (merge as f64).clamp(0.0, 1.0);

        let threshold = (threshold as f64).clamp(0.0, 1.0);
        let valley_shape = (valley_shape as f64).clamp(0.0, 1.0);
        let river_depth = (river_depth as f64).clamp(0.0, 1.0);
        let bank_height = (bank_height as f64).clamp(0.0, 0.5);
        // Widths are authored at a 1024px reference and scaled to the actual
        // output size (mask mode only).
        let river_width_px = scale_to_resolution(river_width.clamp(0, 128) as f32, width as u32, height as u32).round() as f64;
        let valley_width_px = scale_to_resolution(valley_width.clamp(1, 512) as f32, width as u32, height as u32).max(1.0) as f64;

        let w = width as usize;
        let h = height as usize;
        let n = w * h;

        // Splat hills and normalize BEFORE any compositing, so the channel
        // bed/ramp is computed against a stable [0, 1] hill field rather than
        // shifting with whatever the composite's own min/max happens to be
        // (the composite itself is never re-normalized).
        let hills01 = splat_hills_normalized(seed as u32, width, height, density, size, size_variation, height_variation, peakiness, merge);

        if super::is_unconnected(&map_data) {
            return Ok(fallback_response(&hills01, w, h, start_time));
        }

        let g = super::guidance_map_to_grid(&map_data, w, h);

        // Normalized falloff distance d: 0 at/inside the channel, 1 at (or
        // past) the valley rim.
        let d: Vec<f64> = if is_distance_field {
            // Bed follows the field's own shape rather than being perfectly
            // flat - that's the point of feeding in a precomputed field.
            g.iter().map(|&gv| (1.0 - gv).clamp(0.0, 1.0)).collect()
        } else {
            let on: Vec<bool> = g.iter().map(|&gv| gv >= threshold).collect();
            if !on.iter().any(|&b| b) {
                return Ok(fallback_response(&hills01, w, h, start_time));
            }
            let (d2, _) = super::distance_field_labeled(&on, w, h);
            d2.iter().map(|&d2v| ((d2v.sqrt() - river_width_px) / valley_width_px).clamp(0.0, 1.0)).collect()
        };

        // Composite: r = river depth, b = bank height, q = valley wall
        // exponent (1 = V, up to 3 = flat-floored U).
        let r = river_depth;
        let b = bank_height;
        let q = 1.0 + 2.0 * valley_shape;
        let pi = std::f64::consts::PI;

        let mut height_image = FloatImage::new(w as u32, h as u32, 1);
        let mut mask_image = FloatImage::new(w as u32, h as u32, 1);
        for p in 0..n {
            let dv = d[p];
            let ramp = dv.powf(q);
            // smoothstep: hills fade out approaching the channel.
            let hillmod = dv * dv * (3.0 - 2.0 * dv);
            // Hann bump peaking at d=0.25 (mid-bank), zero at d=0 and d>=0.5.
            let levee = 0.5 + 0.5 * (pi * ((dv - 0.25).abs() / 0.25).min(1.0)).cos();
            let composed = (r * ramp + (1.0 - r) * hillmod * hills01[p] + b * levee) / (1.0 + b);

            let x = (p % w) as u32;
            let y = (p / w) as u32;
            height_image.put_pixel(x, y, &[encode(composed as f32)]);
            mask_image.put_pixel(x, y, &[encode((1.0 - ramp) as f32)]);
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(height_image), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(mask_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "guided_rolling_hills_tests.rs"]
mod tests;
