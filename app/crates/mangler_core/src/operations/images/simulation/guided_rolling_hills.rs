//! Guided rolling hills: `rolling hills` parted around a river.
//!
//! Splats the same Hann-kernel hills as the `rolling hills` noise node, but
//! when a river mask is connected (dark = river on a light background) each
//! individual hill's amplitude is scaled by the valley wall height at its own
//! center: hills sitting in the channel vanish, hills on the walls shrink but
//! keep their full dome shape, and hills past the valley rim are untouched.
//! The river then reads as flowing BETWEEN the hills instead of through a
//! faded apron of half-height ghost hills. A narrow smoothstep bank cut trims
//! hill skirts that would spill into the channel, the channel bed stays
//! exactly flat, and a convex wall ramp (steepest at the bank, rounding off
//! into the hilltops) carries the large-scale valley shape.
//!
//! Heuristic composition, not a physical model. Unconnected (or an all-light
//! mask with no pixel at or below the threshold) falls back to plain rolling
//! hills and tiles; once a mask drives the valley the output does NOT tile
//! (the distance transform is not toroidal), matching `carve_river`.

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

/// Grid snap + neighbor search radius shared by the splat loop and the
/// amp-table builder, so both agree on the reachable cell-index range.
fn splat_geometry(density: f64, size: f64, size_variation: f64) -> (i32, i32) {
    let grid = density.round().max(1.0) as i32;
    let max_radius = size * (1.0 + size_variation);
    let search = max_radius.ceil() as i32 + 1;
    (grid, search)
}

/// Per-hill amplitude factors, one per UNWRAPPED cell index the splat loop
/// can visit (axis range `-search ..= grid-1+search`), row-major with stride
/// `side = grid + 2*search`. Indexed by unwrapped cells (hashes use the
/// wrapped pair) so each visual instance of a seam-wrapped hill gets the
/// factor for its actual location.
struct CellAmpTable {
    factors: Vec<f64>,
    search: i32,
    side: i32,
}

/// Builds the per-hill amplitude table: each cell's hill center is recomputed
/// exactly as the splat loop does, mapped from cell space to pixel space, and
/// the convex valley wall height `1-(1-d01)^q` at that point becomes the
/// hill's amplitude factor. Nearest-pixel sampling of the distance field is
/// fine: the factor is a per-hill constant and hills span many pixels.
#[allow(clippy::too_many_arguments)]
fn build_cell_amp_table(
    seed: u32,
    grid: i32,
    search: i32,
    d2: &[f64],
    w: usize,
    h: usize,
    river_width_px: f64,
    valley_width_px: f64,
    q: f64,
) -> CellAmpTable {
    let side = grid + 2 * search;
    let mut factors = vec![1.0_f64; (side * side) as usize];
    for cy in -search..=(grid - 1 + search) {
        for cx in -search..=(grid - 1 + search) {
            let wx = wrap_cell(cx, grid);
            let wy = wrap_cell(cy, grid);
            // Must mirror the splat loop's center formula exactly.
            let kx = cx as f64 + cell_hash(wx, wy, seed, 0);
            let ky = cy as f64 + cell_hash(wx, wy, seed, 1);
            // Cell space -> pixel space (inverse of the loop's px/w*grid),
            // clamped so border-cell hills sample the nearest edge pixel.
            let px = ((kx / grid as f64) * w as f64).clamp(0.0, (w - 1) as f64) as usize;
            let py = ((ky / grid as f64) * h as f64).clamp(0.0, (h - 1) as f64) as usize;
            let d01 = ((d2[py * w + px].sqrt() - river_width_px) / valley_width_px).clamp(0.0, 1.0);
            factors[((cy + search) * side + (cx + search)) as usize] = 1.0 - (1.0 - d01).powf(q);
        }
    }
    CellAmpTable { factors, search, side }
}

/// Splats one Hann-kernel hill per jittered grid cell and min/max normalizes
/// to [0, 1]. The unmodulated path (`amp_table` = None) is a verbatim port of
/// `noise::cellular::rolling_hills`'s splat loop and normalization
/// (`rolling_hills.rs:118-194`) so an unconnected guidance map reproduces
/// that node's output pixel-for-pixel; if `rolling_hills.rs` changes, update
/// this copy to match.
///
/// With an amp table, each hill's contribution is additionally scaled by its
/// per-cell factor, and the result is normalized by the UNMODULATED field's
/// min/max — renormalizing by the modulated field's own stats would brighten
/// everything whenever the tallest hill sits near the river, cancelling the
/// modulation; using the unmodulated stats also keeps pixels beyond the
/// valley rim bit-identical to plain rolling hills.
#[allow(clippy::too_many_arguments)]
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
    amp_table: Option<&CellAmpTable>,
) -> Vec<f64> {
    let (grid, search) = splat_geometry(density, size, size_variation);

    let w = width as usize;
    let h = height as usize;

    // Copy + Send + Sync tuple so the rayon `move` closures can copy it.
    let table: Option<(&[f64], i32, i32)> = amp_table.map(|t| (&t.factors[..], t.search, t.side));

    let pairs: Vec<(f64, f64)> = (0..h).into_par_iter().flat_map_iter(move |py| {
        (0..w).map(move |px| {
            let gx = (px as f64 / w as f64) * grid as f64;
            let gy = (py as f64 / h as f64) * grid as f64;

            let cell_x = gx.floor() as i32;
            let cell_y = gy.floor() as i32;

            let mut sum = 0.0_f64;
            let mut tallest = 0.0_f64;
            let mut msum = 0.0_f64;
            let mut mtallest = 0.0_f64;

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

                    if let Some((factors, s, side)) = table {
                        // Tallest-wins must run on the SCALED contributions:
                        // after modulation the tallest visible hill can be a
                        // different hill than the unmodulated tallest.
                        let scaled = contribution * factors[((cell_y + dy + s) * side + (cell_x + dx + s)) as usize];
                        msum += scaled;
                        mtallest = mtallest.max(scaled);
                    }
                }
            }

            (
                tallest + merge * (sum - tallest),
                mtallest + merge * (msum - mtallest),
            )
        })
    }).collect();

    let min_val = pairs.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let max_val = pairs.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;

    let modulated = amp_table.is_some();
    pairs.iter().map(|&(u, m)| {
        if range < 1e-12 {
            0.5
        } else if modulated {
            // Channel hills can drop below the unmodulated minimum.
            ((m - min_val) / range).clamp(0.0, 1.0)
        } else {
            (u - min_val) / range
        }
    }).collect()
}

/// Builds the plain-hills fallback response used both when the guidance map
/// is unconnected and when a connected mask has no river (dark) pixels: the
/// height output is `hills01` encoded (pixel-identical to plain rolling
/// hills), and the channel mask is all black.
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

/// Operation that generates rolling hills parted around a river mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageSimulationGuidedRollingHills {}

impl OpImageSimulationGuidedRollingHills {
    /// Returns the node metadata (name, description, help) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "guided rolling hills".to_string(),
            description: "Rolling hills parted around a river mask (dark = river): hills shrink and vanish near the channel so the river flows between them, with a flat bed and convex valley walls.".to_string(),
            help: "Heuristic composition, not a physical model: the same Hann-splat hill scatter as the rolling hills node (see that node's help), shaped around a river mask. The mask is DARK = river on a light background - paint the river black on white, or invert the meander node's river mask first. Pixels at or below mask threshold become the channel, widened by river width; valley width sets how far the valley walls climb back to full hill height, both authored as pixels at a 1024px reference and scaled to the output size.\n\nInstead of fading the whole hill field, each individual hill is scaled by the valley wall height at its own center: hills sitting in the channel vanish, hills on the walls shrink but keep their full dome shape, and hills past the rim are untouched - so the river flows between the hills rather than through a faded apron. A narrow bank cut trims hill skirts that would spill into the channel, and the channel bed itself stays exactly flat.\n\nValley shape controls how convex the walls are: 0 = straight V walls, 1 = strongly rounded walls that drop steeply at the bank and ease off into the hilltops. River depth is how much of the relief the valley ramp claims versus the hills; bank height adds a levee bump peaking partway up the bank (0 for no levee). Density/size/size variation/height variation/peakiness/merge are the same hill-shape controls as rolling hills. Deterministic from seed.\n\nUnconnected (or a connected mask with no pixels at or below threshold), the output is pixel-identical to plain rolling hills and tiles; once a mask drives the valley the output does NOT tile (the distance transform is not toroidal, same as carve river).\n\nOutputs the composed heightmap plus a channel mask (bright in the channel, fading to black at the valley rim, all black when nothing is connected) - a ready-made wetness/water mask aligned with the carve.".to_string(),
        }
    }

    /// Creates the default inputs in the simulation convention: seed and
    /// dimensions, the optional river mask and its channel/valley params,
    /// then the hill-shape params (copied verbatim from rolling hills) last.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for hill placement and sizes; change to rearrange the hills."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("guidance map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional river mask, dark = river (paint the river black on white; invert the meander node's river mask first). Unconnected behaves exactly like plain rolling hills."),
            Input::new("mask threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Brightness at or below which a mask pixel counts as river."),
            Input::new("river width".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 128.0)), speed: None }), None)
                .with_description("Widens thin mask paths into a channel, in pixels at a 1024px reference (scales with output size)."),
            Input::new("valley width".to_string(), Value::Integer(160), Some(InputSettings::DragValue { clamp: Some((1.0, 512.0)), speed: None }), None)
                .with_description("Width of the valley walls climbing from the channel back to full hill height, in pixels at a 1024px reference (scales with output size)."),
            Input::new("valley shape".to_string(), Value::Decimal(0.07), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How convex the valley walls are: 0 = straight V walls, 1 = strongly rounded walls that drop steeply at the bank and ease off into the hilltops."),
            Input::new("river depth".to_string(), Value::Decimal(0.08), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Fraction of the relief claimed by the valley ramp versus the hills; higher sinks the channel deeper relative to the surrounding hills."),
            Input::new("bank height".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.01), clamp_to_range: true }), None)
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
    /// 1. Unconnected map -> plain hills fallback (splat with no modulation).
    /// 2. Thresholds the mask (dark = river); no river pixels -> same
    ///    fallback.
    /// 3. Builds the squared-distance field from the channel sites, then the
    ///    per-hill amplitude table (each hill's factor is the convex wall
    ///    height at its center), and splats the hills with that modulation,
    ///    normalized by the unmodulated field's min/max.
    /// 4. Composites per pixel: convex valley wall ramp + bank-cut hills +
    ///    levee bump into the final height, plus the channel mask from the
    ///    wall alone.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let map_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let river_width_converted = convert_input(inputs, 5, ValueType::Integer, &mut input_errors);
        let valley_width_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);
        let valley_shape_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let river_depth_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let bank_height_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let density_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let size_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);
        let size_var_converted = convert_input(inputs, 12, ValueType::Decimal, &mut input_errors);
        let height_var_converted = convert_input(inputs, 13, ValueType::Decimal, &mut input_errors);
        let peakiness_converted = convert_input(inputs, 14, ValueType::Decimal, &mut input_errors);
        let merge_converted = convert_input(inputs, 15, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
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
        // output size.
        let river_width_px = scale_to_resolution(river_width.clamp(0, 128) as f32, width as u32, height as u32).round() as f64;
        let valley_width_px = scale_to_resolution(valley_width.clamp(1, 512) as f32, width as u32, height as u32).max(1.0) as f64;

        let w = width as usize;
        let h = height as usize;
        let n = w * h;

        if super::is_unconnected(&map_data) {
            let hills01 = splat_hills_normalized(seed as u32, width, height, density, size, size_variation, height_variation, peakiness, merge, None);
            return Ok(fallback_response(&hills01, w, h, start_time));
        }

        let g = super::guidance_map_to_grid(&map_data, w, h);
        // Dark = river: pixels at or below the threshold are channel sites.
        let on: Vec<bool> = g.iter().map(|&gv| gv <= threshold).collect();
        if !on.iter().any(|&b| b) {
            let hills01 = splat_hills_normalized(seed as u32, width, height, density, size, size_variation, height_variation, peakiness, merge, None);
            return Ok(fallback_response(&hills01, w, h, start_time));
        }
        let (d2, _) = super::distance_field_labeled(&on, w, h);

        // q = valley wall exponent (1 = straight V walls, up to 3 = strongly
        // convex: steepest at the bank, rounding off into the hilltops).
        let q = 1.0 + 2.0 * valley_shape;
        let (grid, search) = splat_geometry(density, size, size_variation);
        let table = build_cell_amp_table(seed as u32, grid, search, &d2, w, h, river_width_px, valley_width_px, q);
        let hills01 = splat_hills_normalized(seed as u32, width, height, density, size, size_variation, height_variation, peakiness, merge, Some(&table));

        // Composite: r = river depth, b = bank height.
        let r = river_depth;
        let b = bank_height;
        // Narrow bank cut trimming hill skirts that spill into the channel;
        // much narrower than the valley so it reads as a cut bank, not a
        // fade apron. Smoothstep (C1 at both ends) so no crease line shows
        // under normal-from-height.
        let feather_px = (0.1 * valley_width_px).max(2.0);
        let pi = std::f64::consts::PI;

        let mut height_image = FloatImage::new(w as u32, h as u32, 1);
        let mut mask_image = FloatImage::new(w as u32, h as u32, 1);
        for p in 0..n {
            let dist = d2[p].sqrt();
            let dv = ((dist - river_width_px) / valley_width_px).clamp(0.0, 1.0);
            // Convex wall profile: steepest right at the bank, easing off
            // toward the rim, so slopes descend into the river like real
            // hillsides instead of flattening out (concave) near the water.
            let wall = 1.0 - (1.0 - dv).powf(q);
            let ct = ((dist - river_width_px) / feather_px).clamp(0.0, 1.0);
            let cut = ct * ct * (3.0 - 2.0 * ct);
            // Hann bump peaking at d=0.25 (mid-bank), zero at d=0 and d>=0.5.
            let levee = 0.5 + 0.5 * (pi * ((dv - 0.25).abs() / 0.25).min(1.0)).cos();
            let composed = (r * wall + (1.0 - r) * cut * hills01[p] + b * levee) / (1.0 + b);

            let x = (p % w) as u32;
            let y = (p / w) as u32;
            height_image.put_pixel(x, y, &[encode(composed as f32)]);
            mask_image.put_pixel(x, y, &[encode((1.0 - wall) as f32)]);
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
