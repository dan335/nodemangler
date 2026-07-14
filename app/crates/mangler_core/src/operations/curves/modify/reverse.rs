//! Curve reverse modifier: flips point traversal order.
//!
//! Structure-preserving — reverses `points` and, since a Bezier anchor's
//! handle offset is symmetric (out-handle = anchor + h, in-handle = anchor -
//! h), reverses and negates `handles` so the tangents stay geometrically
//! correct under the reversed traversal direction. `interpolation` and
//! `closed` are unchanged. `handles` is reversed exactly as stored (not
//! materialized first), so applying reverse twice is always an exact
//! identity, including for a curve with no explicit handles.

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "reverse_tests.rs"]
mod tests;

/// Reverses `curve`'s point order and its handle vectors (reversed, then
/// each negated). Fewer than 2 points passes through unchanged.
pub(crate) fn reverse_curve(curve: &Curve) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let mut points = curve.points.clone();
    points.reverse();

    let mut handles = curve.handles.clone();
    handles.reverse();
    for h in handles.iter_mut() {
        h[0] = -h[0];
        h[1] = -h[1];
    }

    Curve { points, closed: curve.closed, interpolation: curve.interpolation, handles }
}

/// Operation that reverses a curve's point traversal order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyReverse {}

impl OpCurveModifyReverse {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "reverse".to_string(),
            description: "Reverses a curve's point traversal order.".to_string(),
            help: "Structure-preserving: reverses the order of the control points (and, in Bezier mode, negates and reverses the tangent handles so they stay geometrically correct) without changing the curve's shape, interpolation mode, or open/closed state. Useful before join's auto-orient, or wherever a downstream node cares about traversal direction (e.g. scatter on curve's tangent alignment). Applying reverse twice always returns the original curve exactly.".to_string(),
        }
    }

    /// Creates the default inputs: curve.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to reverse."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The reversed curve."),
        ]
    }

    /// Reverses the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };

        let out = reverse_curve(&curve);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
