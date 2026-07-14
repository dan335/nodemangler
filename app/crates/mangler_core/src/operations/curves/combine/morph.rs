//! Curve morph combiner: pointwise interpolation between two curves.
//!
//! Flattening: flattens both curves, resamples each to a matching point
//! count (the larger of the two curves' control-point counts, in count
//! mode: `ds = length / (count-1)`), then linearly interpolates the two
//! point arrays by `factor`. Emits a `Linear` curve.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{drop_closing_duplicate, flatten_f64, linear_curve, polyline_length, resample, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "morph_tests.rs"]
mod tests;

/// Morphs from `a` to `b` by `factor` (0 = a, 1 = b). Both are flattened and
/// resampled to `target_n = max(a.points.len(), b.points.len())` (clamped
/// into `[2, MAX_OUTPUT_POINTS]`) before a pointwise lerp. Output is closed
/// only when both inputs are closed. Either side with fewer than 2 points
/// returns the other curve unchanged.
pub(crate) fn morph_curves(a: &Curve, b: &Curve, factor: f64) -> Curve {
    if a.points.len() < 2 {
        return b.clone();
    }
    if b.points.len() < 2 {
        return a.clone();
    }
    let factor = factor.clamp(0.0, 1.0);
    let target_n = a.points.len().max(b.points.len()).clamp(2, MAX_OUTPUT_POINTS);

    let poly_a = flatten_f64(a, 48);
    let poly_b = flatten_f64(b, 48);
    let len_a = polyline_length(&poly_a);
    let len_b = polyline_length(&poly_b);
    let ds_a = (len_a / (target_n - 1) as f64).max(1e-9);
    let ds_b = (len_b / (target_n - 1) as f64).max(1e-9);

    let mut ra = Vec::new();
    resample(&poly_a, ds_a, target_n, &mut ra);
    let mut rb = Vec::new();
    resample(&poly_b, ds_b, target_n, &mut rb);

    let n = ra.len().min(rb.len());
    let mut points: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            [
                (ra[i][0] + (rb[i][0] - ra[i][0]) * factor) as f32,
                (ra[i][1] + (rb[i][1] - ra[i][1]) * factor) as f32,
            ]
        })
        .collect();

    let closed = a.closed && b.closed;
    drop_closing_duplicate(&mut points, closed);
    linear_curve(points, closed)
}

/// Operation that morphs between two curves by pointwise interpolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveCombineMorph {}

impl OpCurveCombineMorph {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "morph".to_string(),
            description: "Interpolates between two curves point by point.".to_string(),
            help: "Flattens both curves and resamples each to the same point count (the larger of the two curves' control-point counts), then linearly interpolates every corresponding pair of points by factor (0 = curve a, 1 = curve b, 0.5 = the midpoint shape). Works best when the two curves have a similar overall arc-length parameterization (e.g. both traced left-to-right) - wildly different shapes or orientations can produce a crossing, twisted-looking blend since there's no correspondence beyond matching arc-length fraction.\n\nOutput is closed only when both inputs are closed; otherwise it's an open Linear curve. Either curve with fewer than 2 points is skipped and the other curve is returned unchanged.".to_string(),
        }
    }

    /// Creates the default inputs: curve a, curve b, factor.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve a".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve at factor = 0."),
            Input::new("curve b".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve at factor = 1."),
            Input::new("factor".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Interpolation factor between curve a (0) and curve b (1)."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The morphed Linear curve."),
        ]
    }

    /// Morphs between the two curves from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Curve, &mut input_errors);
        let factor_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Curve(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(factor) = factor_converted.unwrap() else { unreachable!() };

        let out = morph_curves(&a, &b, factor as f64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
