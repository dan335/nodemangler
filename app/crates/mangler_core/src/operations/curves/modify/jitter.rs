//! Curve jitter modifier: seeded per-point normal noise.
//!
//! Flattening: flattens and resamples to uniform spacing (so the noise
//! density is even), then displaces each point along its vertex normal by a
//! seeded uniform random amount, and emits a `Linear` curve.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{drop_closing_duplicate, flatten_f64, linear_curve, resample, vertex_tangent, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "jitter_tests.rs"]
mod tests;

/// Resamples `curve` to `spacing_norm`, then displaces each point along its
/// vertex normal by a seeded uniform random amount in `+-amount_norm`. When
/// `preserve_endpoints` is set and the curve is open, the first and last
/// points are left untouched. Fewer than 2 points passes through unchanged.
pub(crate) fn jitter_curve(curve: &Curve, seed: i32, amount_norm: f64, spacing_norm: f64, preserve_endpoints: bool) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let poly = flatten_f64(curve, 48);
    let mut resampled = Vec::new();
    resample(&poly, spacing_norm.max(1e-9), MAX_OUTPUT_POINTS, &mut resampled);
    drop_closing_duplicate(&mut resampled, curve.closed);

    let n = resampled.len();
    let mut rng = fastrand::Rng::with_seed(seed as u64);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // Always draw, even for a pinned point, so the random stream doesn't
        // shift when preserve_endpoints is toggled.
        let r = (rng.f64() * 2.0 - 1.0) * amount_norm;
        let pinned = preserve_endpoints && !curve.closed && (i == 0 || i == n - 1);
        if pinned {
            out.push(resampled[i]);
        } else {
            let t = vertex_tangent(&resampled, i);
            let normal = [-t[1], t[0]];
            out.push([resampled[i][0] + r * normal[0], resampled[i][1] + r * normal[1]]);
        }
    }
    let points: Vec<[f32; 2]> = out.iter().map(|p| [p[0] as f32, p[1] as f32]).collect();
    linear_curve(points, curve.closed)
}

/// Operation that jitters a curve with seeded per-point normal noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyJitter {}

impl OpCurveModifyJitter {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "jitter".to_string(),
            description: "Adds seeded random noise perpendicular to a curve.".to_string(),
            help: "Resamples the curve to uniform spacing, then displaces each point along its local normal by a random amount in +-amount (uniform, seeded). Useful for roughing up a clean generated curve (ellipse, polygon, ...) into something hand-drawn-looking. Deterministic for a given seed - vary the seed for a different jitter pattern.\n\namount and spacing are authored as pixels at a 1024px reference and divided by 1024 into normalized curve-space units. preserve endpoints, when the curve is open, keeps the first and last points exactly in place (has no effect on closed curves, which have no endpoints). Output is always a Linear curve.".to_string(),
        }
    }

    /// Creates the default inputs: curve, seed, amount, spacing, preserve endpoints.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to jitter."),
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed; vary it for a different jitter pattern."),
            Input::new("amount".to_string(), Value::Decimal(4.0), Some(InputSettings::Slider { range: (0.0, 64.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Maximum perpendicular displacement, in pixels at a 1024px reference (divided by 1024 into normalized units)."),
            Input::new("spacing".to_string(), Value::Decimal(8.0), Some(InputSettings::Slider { range: (1.0, 256.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Point spacing used for resampling before jittering, in pixels at a 1024px reference."),
            Input::new("preserve endpoints".to_string(), Value::Bool(true), None, None)
                .with_description("When the curve is open, keep the first and last points exactly in place."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The jittered Linear curve."),
        ]
    }

    /// Jitters the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let seed_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let amount_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let spacing_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let preserve_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spacing) = spacing_converted.unwrap() else { unreachable!() };
        let Value::Bool(preserve_endpoints) = preserve_converted.unwrap() else { unreachable!() };

        let amount_norm = (amount as f64).clamp(0.0, 64.0) / 1024.0;
        let spacing_norm = (spacing as f64).clamp(1.0, 256.0) / 1024.0;

        let out = jitter_curve(&curve, seed, amount_norm, spacing_norm, preserve_endpoints);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
