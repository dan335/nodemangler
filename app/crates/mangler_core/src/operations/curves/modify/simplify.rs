//! Curve simplify modifier: Ramer-Douglas-Peucker decimation.
//!
//! Flattening: flattens to an `f64` polyline, decimates with
//! `common::rdp_decimate`, and emits a `Linear` curve. Points whose
//! perpendicular deviation from the simplified chord is within `tolerance`
//! are dropped.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{drop_closing_duplicate, flatten_f64, linear_curve, rdp_decimate, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "simplify_tests.rs"]
mod tests;

/// Flattens `curve` and decimates it with Ramer-Douglas-Peucker at
/// `tolerance_norm` (normalized curve-space units), emitting a `Linear`
/// curve. Fewer than 2 points passes through unchanged.
pub(crate) fn simplify_curve(curve: &Curve, tolerance_norm: f64) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let poly = flatten_f64(curve, 48);
    let mut kept = rdp_decimate(&poly, tolerance_norm.max(0.0), MAX_OUTPUT_POINTS);
    let mut kept_f64: Vec<[f64; 2]> = kept.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    drop_closing_duplicate(&mut kept_f64, curve.closed);
    kept = kept_f64.iter().map(|p| [p[0] as f32, p[1] as f32]).collect();
    linear_curve(kept, curve.closed)
}

/// Operation that simplifies a curve via Ramer-Douglas-Peucker decimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifySimplify {}

impl OpCurveModifySimplify {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "simplify".to_string(),
            description: "Reduces a curve's point count via Ramer-Douglas-Peucker decimation.".to_string(),
            help: "Flattens the curve to a polyline and repeatedly drops the point that deviates least from the straight chord spanning its neighbors, stopping once every remaining point deviates by more than `tolerance` from the simplified shape. Useful for cleaning up a dense curve (e.g. from meander or trace contour) before further editing or rasterizing.\n\ntolerance is authored as pixels at a 1024px reference and divided by 1024 into normalized curve-space units, so the same value simplifies proportionally at any scale. Output is always a Linear curve; closed rings keep their closed flag without repeating the seam point.".to_string(),
        }
    }

    /// Creates the default inputs: curve, tolerance.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to simplify."),
            Input::new("tolerance".to_string(), Value::Decimal(2.0), Some(InputSettings::Slider { range: (0.1, 64.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Maximum allowed deviation from the original shape, in pixels at a 1024px reference (divided by 1024 into normalized units)."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The simplified Linear curve."),
        ]
    }

    /// Simplifies the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let tolerance_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tolerance) = tolerance_converted.unwrap() else { unreachable!() };

        let tolerance_norm = (tolerance as f64).clamp(0.1, 64.0) / 1024.0;

        let out = simplify_curve(&curve, tolerance_norm);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
