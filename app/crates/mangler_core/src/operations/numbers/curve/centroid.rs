//! Curve arc-length-weighted centroid.
//!
//! Reports the mean position along a curve's flattened polyline, weighting
//! each segment's midpoint by its length so the result doesn't bunch toward
//! wherever the curve happens to carry a denser cluster of samples.

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::flatten_f64;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Arc-length-weighted mean position of a flattened polyline. Fewer than 2
/// points returns the single point's coordinates (or `(0.5, 0.5)` when
/// empty).
pub(crate) fn weighted_centroid(poly: &[[f64; 2]]) -> [f64; 2] {
    match poly.len() {
        0 => [0.5, 0.5],
        1 => poly[0],
        _ => {
            let mut sum_w = 0.0f64;
            let mut sum_x = 0.0f64;
            let mut sum_y = 0.0f64;
            for seg in poly.windows(2) {
                let dx = seg[1][0] - seg[0][0];
                let dy = seg[1][1] - seg[0][1];
                let w = (dx * dx + dy * dy).sqrt();
                let mx = (seg[0][0] + seg[1][0]) * 0.5;
                let my = (seg[0][1] + seg[1][1]) * 0.5;
                sum_w += w;
                sum_x += mx * w;
                sum_y += my * w;
            }
            if sum_w <= 1e-12 {
                // All segments zero-length (coincident points): fall back to
                // the plain mean of the points.
                let n = poly.len() as f64;
                let (mut mx, mut my) = (0.0, 0.0);
                for p in poly {
                    mx += p[0];
                    my += p[1];
                }
                [mx / n, my / n]
            } else {
                [sum_x / sum_w, sum_y / sum_w]
            }
        }
    }
}

/// Operation that computes the arc-length-weighted centroid of a curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCurveCentroid {}

impl OpNumberCurveCentroid {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "centroid".to_string(),
            description: "Finds the arc-length-weighted mean position of a curve.".to_string(),
            help: "Flattens the curve and averages the midpoint of every segment, weighted by that segment's length, so the result is the curve's balance point along its own path rather than a plain average of however many samples it flattens into. Values are in normalized [0,1]² units.\n\nFewer than 2 points falls back to that single point's coordinates; an empty curve (no points) falls back to (0.5, 0.5), the center of the unit square.".to_string(),
        }
    }

    /// Creates the input port: a single curve to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve whose centroid is measured."),
        ]
    }

    /// Creates the output ports: x, y.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.5), None)
                .with_description("Centroid x in normalized [0,1] units."),
            Output::new("y".to_string(), Value::Decimal(0.5), None)
                .with_description("Centroid y in normalized [0,1] units."),
        ]
    }

    /// Executes the centroid computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };

        let poly = flatten_f64(&curve, 48);
        let [cx, cy] = weighted_centroid(&poly);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(cx as f32) },
                OutputResponse { value: Value::Decimal(cy as f32) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "centroid_tests.rs"]
mod tests;
