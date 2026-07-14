//! Curve trim modifier: extracts an arc-length span.
//!
//! Flattening: flattens (a closed curve is cut open starting at arc 0, i.e.
//! its own first control point, since that's where the flattened polyline
//! always begins), finds the two cut points by interpolating along the
//! cumulative arc length, and keeps everything strictly between them. Output
//! is always an open `Linear` curve, even when the input was closed.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{cumulative_arc, flatten_f64, linear_curve, rdp_decimate, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "trim_tests.rs"]
mod tests;

/// Interpolates the point on `poly` at cumulative arc length `target`, given
/// `arc` (from [`cumulative_arc`]). Clamps to the first/last point outside
/// `[arc[0], arc.last()]`.
fn point_at_arc(poly: &[[f64; 2]], arc: &[f64], target: f64) -> [f64; 2] {
    let n = poly.len();
    if n == 0 {
        return [0.0, 0.0];
    }
    if target <= arc[0] {
        return poly[0];
    }
    if target >= *arc.last().unwrap() {
        return poly[n - 1];
    }
    for i in 0..n - 1 {
        if target >= arc[i] && target <= arc[i + 1] {
            let seg_len = arc[i + 1] - arc[i];
            let t = if seg_len > 1e-12 { (target - arc[i]) / seg_len } else { 0.0 };
            return [
                poly[i][0] + t * (poly[i + 1][0] - poly[i][0]),
                poly[i][1] + t * (poly[i + 1][1] - poly[i][1]),
            ];
        }
    }
    poly[n - 1]
}

/// Extracts the span of `curve` between normalized arc-length parameters
/// `t0`/`t1` (each clamped to `[0,1]`, swapped if `t0 > t1`), interpolating
/// the cut points. Fewer than 2 points passes through unchanged.
pub(crate) fn trim_curve(curve: &Curve, t0: f64, t1: f64) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let mut t0 = t0.clamp(0.0, 1.0);
    let mut t1 = t1.clamp(0.0, 1.0);
    if t0 > t1 {
        std::mem::swap(&mut t0, &mut t1);
    }

    let poly = flatten_f64(curve, 48);
    let mut arc = Vec::new();
    cumulative_arc(&poly, &mut arc);
    let total = *arc.last().unwrap_or(&0.0);
    if total <= 1e-12 {
        // Degenerate (all points coincident): nothing to trim, just clamp
        // to at least 2 points so the overlay floor is respected.
        let p = poly[0];
        return linear_curve(vec![[p[0] as f32, p[1] as f32], [p[0] as f32, p[1] as f32]], false);
    }

    let target0 = t0 * total;
    let target1 = t1 * total;
    let p0 = point_at_arc(&poly, &arc, target0);
    let p1 = point_at_arc(&poly, &arc, target1);

    let mut out = vec![p0];
    for (i, &a) in arc.iter().enumerate() {
        if a > target0 && a < target1 {
            out.push(poly[i]);
        }
    }
    out.push(p1);

    let capped = rdp_decimate(&out, 1e-9, MAX_OUTPUT_POINTS);
    linear_curve(capped, false)
}

/// Operation that extracts an arc-length span of a curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyTrim {}

impl OpCurveModifyTrim {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "trim".to_string(),
            description: "Extracts a span of a curve between two arc-length fractions.".to_string(),
            help: "Flattens the curve and keeps only the span between `start t` and `end t` (normalized arc-length fractions, 0 = curve start, 1 = curve end), interpolating the two cut points so the endpoints land exactly on the requested fraction rather than snapping to the nearest sample. A closed curve is cut open starting at its own first control point (arc 0); the output is always an open Linear curve. Values are swapped automatically if start t is greater than end t.".to_string(),
        }
    }

    /// Creates the default inputs: curve, start t, end t.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to trim."),
            Input::new("start t".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Normalized arc-length fraction where the trimmed span starts."),
            Input::new("end t".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Normalized arc-length fraction where the trimmed span ends."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The trimmed open Linear curve."),
        ]
    }

    /// Trims the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let t0_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let t1_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t0) = t0_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t1) = t1_converted.unwrap() else { unreachable!() };

        let out = trim_curve(&curve, t0 as f64, t1 as f64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
