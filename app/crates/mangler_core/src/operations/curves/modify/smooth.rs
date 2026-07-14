//! Curve smooth modifier: Chaikin corner-cutting or Laplacian averaging.
//!
//! Unlike most modifiers, this one works directly on the curve's *control
//! points* rather than the flattened polyline (cutting corners of a
//! low-point-count curve is the point; flattening first would just smooth
//! the already-smooth interpolation instead). It still emits a `Linear`
//! curve, decimated back under `common::MAX_OUTPUT_POINTS` since repeated
//! Chaikin subdivision roughly doubles the point count each iteration.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{laplacian_smooth_once, linear_curve, rdp_decimate, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "smooth_tests.rs"]
mod tests;

/// One Chaikin corner-cutting pass: each edge `(a, b)` is replaced by the two
/// points at 1/4 and 3/4 along it. Open curves pin both endpoints (the first
/// and last points are kept exactly, and only the interior cut points are
/// inserted); closed curves cut every edge, including the wrap-around one.
/// Fewer than 3 points is a no-op (nothing to cut).
pub(crate) fn chaikin_once(points: &[[f64; 2]], closed: bool) -> Vec<[f64; 2]> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let seg_count = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(seg_count * 2);
    if !closed {
        out.push(points[0]);
    }
    for i in 0..seg_count {
        let a = points[i];
        let b = points[(i + 1) % n];
        let q = [0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]];
        let r = [0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1]];
        if !closed && i == 0 {
            out.push(r);
        } else if !closed && i == seg_count - 1 {
            out.push(q);
        } else {
            out.push(q);
            out.push(r);
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

/// Applies `iterations` passes of the chosen smoothing method to `curve`'s
/// control points (not its flattened polyline), then decimates back under
/// `MAX_OUTPUT_POINTS` and emits a `Linear` curve. Fewer than 2 points passes
/// through unchanged.
pub(crate) fn smooth_curve(curve: &Curve, method: &str, iterations: u32) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let mut points: Vec<[f64; 2]> = curve.points.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    for _ in 0..iterations {
        points = if method == "laplacian" {
            laplacian_smooth_once(&points, curve.closed)
        } else {
            chaikin_once(&points, curve.closed)
        };
    }
    let capped = rdp_decimate(&points, 1e-9, MAX_OUTPUT_POINTS);
    linear_curve(capped, curve.closed)
}

/// Operation that smooths a curve's control points via Chaikin corner-cutting
/// or Laplacian averaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifySmooth {}

impl OpCurveModifySmooth {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "smooth".to_string(),
            description: "Smooths a curve's control points.".to_string(),
            help: "Runs directly on the control points (not the flattened polyline), so it works on sharp low-point curves like a hand-drawn or polygon shape. 'chaikin' repeatedly cuts each corner at the 1/4 and 3/4 points of its two adjacent edges (rounds corners, roughly doubles point count per iteration); 'laplacian' repeatedly moves each point halfway toward the average of its neighbors (relaxes wiggle without changing point count). Open curves keep their first and last points fixed; closed curves smooth all the way around. Output is always a Linear curve, decimated back under the point cap since repeated Chaikin subdivision grows the point count quickly.".to_string(),
        }
    }

    /// Creates the default inputs: curve, method, iterations.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to smooth."),
            Input::new("method".to_string(), Value::Text("chaikin".to_string()), Some(InputSettings::Dropdown {
                options: vec!["chaikin".to_string(), "laplacian".to_string()],
            }), None)
                .with_description("Smoothing method: chaikin (corner-cutting, rounds sharp corners) or laplacian (neighbor averaging, relaxes wiggle)."),
            Input::new("iterations".to_string(), Value::Integer(2), Some(InputSettings::DragValue { clamp: Some((1.0, 8.0)), speed: None }), None)
                .with_description("Number of smoothing passes (1-8). More iterations smooths further."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The smoothed Linear curve."),
        ]
    }

    /// Smooths the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let method_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let iterations_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Text(method) = method_converted.unwrap() else { unreachable!() };
        let Value::Integer(iterations) = iterations_converted.unwrap() else { unreachable!() };

        let iterations = iterations.clamp(1, 8) as u32;

        let out = smooth_curve(&curve, &method, iterations);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
