//! Stamp a pattern image along a curve at even arc-length spacing.
//!
//! Walks the curve at a fixed pixel spacing and drops a copy of the input
//! pattern at each step, optionally rotated to the local tangent, with
//! deterministic per-stamp scale / rotation jitter and positional jitter along
//! and across the curve. Compositing is max blend, sharing [`draw_stamp`] with
//! the `splatter` node.

use crate::curve::Curve;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{cumulative_arc, flatten_f64};
use crate::operations::images::patterns::{draw_stamp, StampPlacement};
use crate::operations::{convert_input, default_image, scale_to_resolution, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
#[path = "scatter_on_curve_tests.rs"]
mod tests;

/// Hard cap on stamp count so a tiny spacing on a long curve can't runaway.
const MAX_STAMPS: usize = 100_000;

/// Advances an LCG state by one step using Knuth's constants.
fn lcg(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

/// Draws a float in `[0,1)` from an LCG state, returning the value and the
/// advanced state.
fn lcg_float(seed: u64) -> (f64, u64) {
    let next = lcg(seed);
    let val = (next >> 33) as f64 / (1u64 << 31) as f64;
    (val, next)
}

/// Position and unit tangent at arc distance `a` along the pixel-space polyline
/// `poly` with cumulative arc lengths `arc`. Clamps to the endpoints.
fn sample_arc(poly: &[[f64; 2]], arc: &[f64], a: f64) -> ([f64; 2], [f64; 2]) {
    let n = poly.len();
    if n == 0 {
        return ([0.0, 0.0], [1.0, 0.0]);
    }
    if n == 1 {
        return (poly[0], [1.0, 0.0]);
    }
    // Find the segment containing `a`.
    let mut i = 0;
    while i + 1 < n - 1 && arc[i + 1] < a {
        i += 1;
    }
    let seg = arc[i + 1] - arc[i];
    let f = if seg > 0.0 { ((a - arc[i]) / seg).clamp(0.0, 1.0) } else { 0.0 };
    let pos = [poly[i][0] + f * (poly[i + 1][0] - poly[i][0]), poly[i][1] + f * (poly[i + 1][1] - poly[i][1])];
    let dx = poly[i + 1][0] - poly[i][0];
    let dy = poly[i + 1][1] - poly[i][1];
    let len = (dx * dx + dy * dy).sqrt();
    let tan = if len > 1e-12 { [dx / len, dy / len] } else { [1.0, 0.0] };
    (pos, tan)
}

/// Operation that stamps a pattern along a curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternScatterOnCurve {}

impl OpImagePatternScatterOnCurve {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "scatter on curve".to_string(),
            description: "Stamps a pattern image along a curve at even spacing.".to_string(),
            help: "Walks the curve at a fixed 'spacing' (pixels at a 1024px reference) and stamps a copy of the pattern at each step, drawn at 'stamp size' pixels. With 'align to curve' on, each stamp rotates to the local tangent direction. Per-stamp jitter is deterministic for a given 'seed': 'scale random' and 'rotation random' vary size and angle, while 'jitter along' / 'jitter across' nudge each stamp along and perpendicular to the curve (pixels at a 1024px reference).\n\nStamps composite with a max blend, so overlaps take the brightest channels; output channel count matches the pattern. An unconnected pattern (the default 1px white image) stamps solid squares. A degenerate curve produces an empty image.".to_string(),
        }
    }

    /// Creates the default inputs (seed first, per repo convention).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(42), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed; same seed always produces the same layout."),
            Input::new("pattern".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image stamped along the curve."),
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to stamp along."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("spacing".to_string(), Value::Decimal(64.0), Some(InputSettings::DragValue { clamp: Some((4.0, 1024.0)), speed: Some(1.0) }), None)
                .with_description("Arc-length distance between stamps in pixels at a 1024px reference; scales with resolution."),
            Input::new("stamp size".to_string(), Value::Decimal(64.0), Some(InputSettings::DragValue { clamp: Some((1.0, 2048.0)), speed: Some(1.0) }), None)
                .with_description("Base stamp size in pixels at a 1024px reference before per-instance random scaling."),
            Input::new("align to curve".to_string(), Value::Bool(true), None, None)
                .with_description("Rotate each stamp to the curve's local tangent direction."),
            Input::new("scale random".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Random variation applied to each stamp's scale."),
            Input::new("rotation random".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Maximum extra random rotation per stamp in degrees, added to the alignment angle."),
            Input::new("jitter along".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: Some((0.0, 512.0)), speed: Some(1.0) }), None)
                .with_description("Random shift along the curve per stamp, pixels at a 1024px reference."),
            Input::new("jitter across".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: Some((0.0, 512.0)), speed: Some(1.0) }), None)
                .with_description("Random shift perpendicular to the curve per stamp, pixels at a 1024px reference."),
        ]
    }

    /// Creates the default output: a single composite image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Composite image with the pattern stamped along the curve using max blending."),
        ]
    }

    /// Stamps the pattern along the curve into a composite image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let pattern_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let curve_converted = convert_input(inputs, 2, ValueType::Curve, &mut input_errors);
        let width_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let spacing_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let stamp_size_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let align_converted = convert_input(inputs, 7, ValueType::Bool, &mut input_errors);
        let scale_random_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let rotation_random_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let jitter_along_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let jitter_across_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Image { data: pattern, change_id: _ } = pattern_converted.unwrap() else { unreachable!() };
        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spacing) = spacing_converted.unwrap() else { unreachable!() };
        let Value::Decimal(stamp_size) = stamp_size_converted.unwrap() else { unreachable!() };
        let Value::Bool(align) = align_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_random) = scale_random_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation_random) = rotation_random_converted.unwrap() else { unreachable!() };
        let Value::Decimal(jitter_along) = jitter_along_converted.unwrap() else { unreachable!() };
        let Value::Decimal(jitter_across) = jitter_across_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let ch = pattern.channels() as usize;

        let spacing_px = (scale_to_resolution(spacing.max(1.0), width as u32, height as u32) as f64).max(1.0);
        let stamp_px = scale_to_resolution(stamp_size.max(1.0), width as u32, height as u32) as f64;
        let scale_random = scale_random.clamp(0.0, 1.0) as f64;
        let rotation_random = (rotation_random as f64).to_radians();
        let jitter_along_px = scale_to_resolution(jitter_along.max(0.0), width as u32, height as u32) as f64;
        let jitter_across_px = scale_to_resolution(jitter_across.max(0.0), width as u32, height as u32) as f64;

        // Flatten to pixel space; arc lengths and stepping are then in pixels.
        let poly_norm = flatten_f64(&curve, 48);
        let poly: Vec<[f64; 2]> = poly_norm
            .iter()
            .map(|p| [p[0] * width as f64, p[1] * height as f64])
            .collect();

        let mut stamps: Vec<StampPlacement> = Vec::new();
        if poly.len() >= 2 {
            let mut arc: Vec<f64> = Vec::new();
            cumulative_arc(&poly, &mut arc);
            let total = *arc.last().unwrap();

            if total > 0.0 {
                let n = (total / spacing_px).floor() as usize;
                let count = (n + 1).min(MAX_STAMPS);
                let mut rng_state = lcg(seed as u64 ^ 0x5CA77E11);
                for k in 0..count {
                    let (rs, s) = lcg_float(rng_state);
                    let (rr, s) = lcg_float(s);
                    let (ja, s) = lcg_float(s);
                    let (jc, s) = lcg_float(s);
                    rng_state = s;

                    let a = (k as f64 * spacing_px).min(total);
                    let (pos, tan) = sample_arc(&poly, &arc, a);
                    let normal = [-tan[1], tan[0]];

                    let off_along = (ja - 0.5) * 2.0 * jitter_along_px;
                    let off_across = (jc - 0.5) * 2.0 * jitter_across_px;
                    let center_x = pos[0] + off_along * tan[0] + off_across * normal[0];
                    let center_y = pos[1] + off_along * tan[1] + off_across * normal[1];

                    let inst_scale = (1.0 - scale_random + rs * scale_random * 2.0).max(0.01);
                    let draw = stamp_px * inst_scale;

                    let base_angle = if align { tan[1].atan2(tan[0]) } else { 0.0 };
                    let angle = base_angle + (rr - 0.5) * 2.0 * rotation_random;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Rotated bounding box, clamped to the output bounds.
                    let half = draw * 0.5;
                    let corners = [(-half, -half), (half, -half), (-half, half), (half, half)];
                    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
                    for (cx_off, cy_off) in &corners {
                        let wx = cos_a * cx_off - sin_a * cy_off + center_x;
                        let wy = sin_a * cx_off + cos_a * cy_off + center_y;
                        min_x = min_x.min(wx);
                        max_x = max_x.max(wx);
                        min_y = min_y.min(wy);
                        max_y = max_y.max(wy);
                    }

                    stamps.push(StampPlacement {
                        center_x,
                        center_y,
                        cos_a,
                        sin_a,
                        draw,
                        tint: [1.0, 1.0, 1.0],
                        sx: (min_x.floor() as i32).max(0),
                        ex: (max_x.ceil() as i32).min(width),
                        sy: (min_y.floor() as i32).max(0),
                        ey: (max_y.ceil() as i32).min(height),
                    });
                }
            }
        }

        // Row-parallel max-blend composite (same structure as splatter).
        let pattern_ref = &pattern;
        let stamps_ref = &stamps;
        let pixels: Vec<f32> = (0..height)
            .into_par_iter()
            .flat_map_iter(move |py| {
                let mut row = vec![0.0f32; width as usize * ch];
                for stamp in stamps_ref {
                    draw_stamp(&mut row, py, stamp, pattern_ref, ch);
                }
                row
            })
            .collect();

        let image = FloatImage::from_raw(width as u32, height as u32, pattern.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } }],
        })
    }
}
