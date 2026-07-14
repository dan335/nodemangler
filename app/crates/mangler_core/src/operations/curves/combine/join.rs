//! Curve join combiner: concatenates two curves end-to-end.
//!
//! When both curves share the same interpolation, their control points (and
//! materialized handles) are concatenated directly, keeping that
//! interpolation. Otherwise both are flattened to polylines and concatenated
//! as a `Linear` curve, decimated back under `common::MAX_OUTPUT_POINTS`.

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{dist, flatten_f64, linear_curve, rdp_decimate, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "join_tests.rs"]
mod tests;

/// Squared-distance-free f32 distance (thin wrapper matching `common::dist`'s
/// signature, but for the `f32` point space `Curve` stores).
fn dist32(a: [f32; 2], b: [f32; 2]) -> f64 {
    dist([a[0] as f64, a[1] as f64], [b[0] as f64, b[1] as f64])
}

/// Joins `a` and `b` end-to-end. If `auto_orient`, `b` is reversed first when
/// its far end (last point) is closer to `a`'s end than its near end (first
/// point) is - minimizing the seam gap. Degenerate (<2 points) side returns
/// the other curve unchanged.
pub(crate) fn join_curves(a: &Curve, b: &Curve, auto_orient: bool, close: bool) -> Curve {
    if a.points.len() < 2 {
        return b.clone();
    }
    if b.points.len() < 2 {
        return a.clone();
    }

    if a.interpolation == b.interpolation {
        let mut a2 = a.clone();
        a2.materialize_handles();
        let mut b2 = b.clone();
        b2.materialize_handles();

        if auto_orient {
            let a_end = *a2.points.last().unwrap();
            let b_start = b2.points[0];
            let b_end = *b2.points.last().unwrap();
            if dist32(a_end, b_end) < dist32(a_end, b_start) {
                b2.points.reverse();
                b2.handles.reverse();
                for h in b2.handles.iter_mut() {
                    h[0] = -h[0];
                    h[1] = -h[1];
                }
            }
        }

        let mut points = a2.points;
        points.extend(b2.points);
        let mut handles = a2.handles;
        handles.extend(b2.handles);

        Curve { points, closed: close, interpolation: a.interpolation, handles }
    } else {
        let mut poly_a = flatten_f64(a, 48);
        let mut poly_b = flatten_f64(b, 48);

        if auto_orient {
            let a_end = *poly_a.last().unwrap();
            let b_start = poly_b[0];
            let b_end = *poly_b.last().unwrap();
            if dist(a_end, b_end) < dist(a_end, b_start) {
                poly_b.reverse();
            }
        }

        poly_a.extend(poly_b);
        let capped = rdp_decimate(&poly_a, 1e-9, MAX_OUTPUT_POINTS);
        linear_curve(capped, close)
    }
}

/// Operation that concatenates two curves end-to-end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveCombineJoin {}

impl OpCurveCombineJoin {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "join".to_string(),
            description: "Concatenates two curves end-to-end.".to_string(),
            help: "Joins curve b onto the end of curve a. When both curves use the same interpolation mode, their control points (and materialized tangent handles) are concatenated directly, keeping that interpolation and the exact point count of a+b; when they differ, both are flattened to polylines first and the result is a Linear curve.\n\nauto orient (default on) reverses b when its far end is closer to a's end than its near end is, minimizing the gap at the seam - turn it off to always join a's end straight into b's first point. close sets the joined curve's closed flag (does not itself add a closing segment beyond what closed rendering already implies). Either curve with fewer than 2 points is skipped and the other curve is returned unchanged.".to_string(),
        }
    }

    /// Creates the default inputs: curve a, curve b, auto orient, close.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve a".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("First curve; b is appended after it."),
            Input::new("curve b".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Second curve, appended after a (possibly reversed - see auto orient)."),
            Input::new("auto orient".to_string(), Value::Bool(true), None, None)
                .with_description("Reverse b first when doing so puts its nearer end next to a's end, minimizing the seam gap."),
            Input::new("close".to_string(), Value::Bool(false), None, None)
                .with_description("Mark the joined curve as closed."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The joined curve."),
        ]
    }

    /// Joins the two curves from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Curve, &mut input_errors);
        let auto_orient_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);
        let close_converted = convert_input(inputs, 3, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Curve(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Bool(auto_orient) = auto_orient_converted.unwrap() else { unreachable!() };
        let Value::Bool(close) = close_converted.unwrap() else { unreachable!() };

        let out = join_curves(&a, &b, auto_orient, close);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
