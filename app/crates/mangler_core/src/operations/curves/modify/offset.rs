//! Curve offset modifier: parallel-curve displacement.
//!
//! Flattening, and a documented heuristic: resamples to a spacing tied to
//! the offset distance, displaces every point along its vertex normal, runs
//! one Laplacian relaxation pass to smooth the zigzag that a naive per-vertex
//! normal offset produces on curved regions, and cleans up with a light RDP
//! pass. This is **not** a true parallel-curve (offset-curve) construction:
//! self-intersections that appear on tight inner offsets are not detected or
//! removed.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{
    drop_closing_duplicate, flatten_f64, laplacian_smooth_once, linear_curve, rdp_decimate, resample, vertex_tangent,
    MAX_OUTPUT_POINTS,
};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "offset_tests.rs"]
mod tests;

/// Offsets `curve` by `distance_norm` along its vertex normals (positive =
/// the perpendicular direction `[-tangent.y, tangent.x]`). Resamples first at
/// `max(2px@1024, |distance|/4)` spacing, then relaxes once and decimates.
/// Fewer than 2 points passes through unchanged. Heuristic: does not detect
/// or remove self-intersections.
pub(crate) fn offset_curve(curve: &Curve, distance_norm: f64) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let poly = flatten_f64(curve, 48);
    let spacing = (2.0 / 1024.0_f64).max(distance_norm.abs() / 4.0);
    let mut resampled = Vec::new();
    resample(&poly, spacing.max(1e-9), MAX_OUTPUT_POINTS, &mut resampled);
    drop_closing_duplicate(&mut resampled, curve.closed);

    let n = resampled.len();
    let mut displaced = Vec::with_capacity(n);
    for i in 0..n {
        let t = vertex_tangent(&resampled, i);
        let normal = [-t[1], t[0]];
        displaced.push([
            resampled[i][0] + distance_norm * normal[0],
            resampled[i][1] + distance_norm * normal[1],
        ]);
    }

    let relaxed = laplacian_smooth_once(&displaced, curve.closed);
    let cleanup_tol = (spacing * 0.05).max(1e-9);
    let kept = rdp_decimate(&relaxed, cleanup_tol, MAX_OUTPUT_POINTS);
    linear_curve(kept, curve.closed)
}

/// Operation that offsets a curve along its normal by a fixed distance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyOffset {}

impl OpCurveModifyOffset {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "offset".to_string(),
            description: "Displaces a curve perpendicular to itself by a fixed distance.".to_string(),
            help: "Heuristic parallel-curve offset: resamples the curve at a spacing tied to the offset distance, pushes every point out along its local normal, relaxes once with a Laplacian pass to smooth the zigzag a naive per-vertex offset leaves on curved sections, then runs a light RDP cleanup pass. Positive distance offsets to one side, negative to the other (the sign flips which side, consistent with the curve's own point order).\n\nThis is not a true offset-curve construction: tight inward offsets (distance larger than the local radius of curvature) can self-intersect, and those intersections are not detected or removed - simplify or manually clean up the result if that happens.\n\ndistance is authored as pixels at a 1024px reference and divided by 1024 into normalized curve-space units. Output is always a Linear curve.".to_string(),
        }
    }

    /// Creates the default inputs: curve, distance.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to offset."),
            Input::new("distance".to_string(), Value::Decimal(8.0), Some(InputSettings::Slider { range: (-128.0, 128.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Offset distance in pixels at a 1024px reference (divided by 1024 into normalized units). Sign chooses the side."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The offset Linear curve."),
        ]
    }

    /// Offsets the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let distance_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Decimal(distance) = distance_converted.unwrap() else { unreachable!() };

        let distance_norm = (distance as f64).clamp(-128.0, 128.0) / 1024.0;

        let out = offset_curve(&curve, distance_norm);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
