//! Curve round corners modifier: quadratic fillets at sharp vertices.
//!
//! Like `smooth`, this works directly on the curve's *control points*
//! (each interior vertex is a "corner" to round), not the flattened
//! polyline. Emits a `Linear` curve, decimated back under
//! `common::MAX_OUTPUT_POINTS`.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{dist, linear_curve, rdp_decimate, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "round_corners_tests.rs"]
mod tests;

/// Samples a quadratic Bezier fillet from `p_in` to `p_out` with `corner` as
/// the (sharp) control point, `samples` points inclusive of both ends.
fn quadratic_fillet(p_in: [f64; 2], corner: [f64; 2], p_out: [f64; 2], samples: usize) -> Vec<[f64; 2]> {
    let denom = (samples.max(2) - 1) as f64;
    (0..samples.max(2))
        .map(|k| {
            let t = k as f64 / denom;
            let u = 1.0 - t;
            [
                u * u * p_in[0] + 2.0 * u * t * corner[0] + t * t * p_out[0],
                u * u * p_in[1] + 2.0 * u * t * corner[1] + t * t * p_out[1],
            ]
        })
        .collect()
}

/// Number of samples per rounded corner (inclusive of both cut points).
const FILLET_SAMPLES: usize = 8;

/// Rounds every interior corner of `points` (every vertex for a closed
/// curve, or every vertex except the two endpoints for an open one) with an
/// 8-sample quadratic fillet, cutting back from the corner by
/// `min(radius, 0.5 * shorter adjacent segment)` along each adjacent edge.
pub(crate) fn round_corners_points(points: &[[f64; 2]], closed: bool, radius: f64) -> Vec<[f64; 2]> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let corner_at = |i: usize| -> Vec<[f64; 2]> {
        let prev = points[(i + n - 1) % n];
        let corner = points[i];
        let next = points[(i + 1) % n];
        let len_in = dist(corner, prev);
        let len_out = dist(corner, next);
        let cutback = radius.min(0.5 * len_in.min(len_out));
        let dir_in = if len_in > 1e-12 { [(prev[0] - corner[0]) / len_in, (prev[1] - corner[1]) / len_in] } else { [0.0, 0.0] };
        let dir_out = if len_out > 1e-12 { [(next[0] - corner[0]) / len_out, (next[1] - corner[1]) / len_out] } else { [0.0, 0.0] };
        let p_in = [corner[0] + cutback * dir_in[0], corner[1] + cutback * dir_in[1]];
        let p_out = [corner[0] + cutback * dir_out[0], corner[1] + cutback * dir_out[1]];
        quadratic_fillet(p_in, corner, p_out, FILLET_SAMPLES)
    };

    let mut out = Vec::new();
    if closed {
        for i in 0..n {
            out.extend(corner_at(i));
        }
    } else {
        out.push(points[0]);
        for i in 1..n - 1 {
            out.extend(corner_at(i));
        }
        out.push(points[n - 1]);
    }
    out
}

/// Rounds `curve`'s control-point corners and decimates the result back
/// under `MAX_OUTPUT_POINTS`, emitting a `Linear` curve. Fewer than 2 points
/// passes through unchanged.
pub(crate) fn round_corners_curve(curve: &Curve, radius_norm: f64) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let points: Vec<[f64; 2]> = curve.points.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    let rounded = round_corners_points(&points, curve.closed, radius_norm.max(0.0));
    let capped = rdp_decimate(&rounded, 1e-9, MAX_OUTPUT_POINTS);
    linear_curve(capped, curve.closed)
}

/// Operation that rounds sharp corners of a curve with quadratic fillets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyRoundCorners {}

impl OpCurveModifyRoundCorners {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "round corners".to_string(),
            description: "Rounds sharp corners of a curve with quadratic fillets.".to_string(),
            help: "Works directly on the control points (each interior vertex is treated as a corner - every vertex for a closed curve, every vertex except the two endpoints for an open one). Each corner is cut back along both adjacent edges by `min(radius, half the shorter edge)` and replaced with an 8-sample quadratic Bezier fillet, so tight corners on short segments round proportionally instead of overshooting past the segment's midpoint. Output is always a Linear curve, decimated back under the point cap.\n\nradius is authored as pixels at a 1024px reference and divided by 1024 into normalized curve-space units.".to_string(),
        }
    }

    /// Creates the default inputs: curve, radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve whose corners to round."),
            Input::new("radius".to_string(), Value::Decimal(8.0), Some(InputSettings::Slider { range: (1.0, 128.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Target fillet radius in pixels at a 1024px reference (divided by 1024 into normalized units); cut back further on short segments."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The rounded Linear curve."),
        ]
    }

    /// Rounds the curve's corners from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let radius_norm = (radius as f64).clamp(1.0, 128.0) / 1024.0;

        let out = round_corners_curve(&curve, radius_norm);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
