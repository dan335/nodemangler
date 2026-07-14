//! Curve resample modifier: uniform-spacing resampling.
//!
//! Flattening: flattens to an `f64` polyline, resamples to uniform spacing
//! via `common::resample` (in either a spacing- or point-count-driven mode),
//! and emits a `Linear` curve. Feeding the flatten a closed curve's polyline
//! *with* its wrap-around duplicate lets the spacing math correctly account
//! for the closing segment's length; the duplicate is dropped from the final
//! output afterward (a closed `Curve` never repeats its seam point).

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{drop_closing_duplicate, flatten_f64, linear_curve, polyline_length, resample as resample_polyline, MAX_OUTPUT_POINTS};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "resample_tests.rs"]
mod tests;

/// Resamples `curve` to uniform spacing. `mode == "count"` targets `count`
/// points (spacing = total length / (count-1)); anything else uses
/// `spacing_norm` directly. Fewer than 2 points passes through unchanged.
pub(crate) fn resample_curve(curve: &Curve, mode: &str, spacing_norm: f64, count: usize) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let poly = flatten_f64(curve, 48);
    let ds = if mode == "count" {
        let total = polyline_length(&poly);
        let n = count.max(2);
        total / (n - 1) as f64
    } else {
        spacing_norm
    };
    let mut out = Vec::new();
    resample_polyline(&poly, ds.max(1e-9), MAX_OUTPUT_POINTS, &mut out);
    drop_closing_duplicate(&mut out, curve.closed);
    let points: Vec<[f32; 2]> = out.iter().map(|p| [p[0] as f32, p[1] as f32]).collect();
    linear_curve(points, curve.closed)
}

/// Operation that resamples a curve to uniform point spacing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyResample {}

impl OpCurveModifyResample {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resample".to_string(),
            description: "Resamples a curve to uniform point spacing.".to_string(),
            help: "Flattens the curve to a polyline and rebuilds it with evenly-spaced points, either at a fixed spacing or targeting a total point count. Spacing widens automatically to respect the internal point cap on pathologically fine requests. Useful before jitter/offset (which need uniform spacing to look right) or to normalize point density before further editing.\n\nspacing is authored as pixels at a 1024px reference and divided by 1024 into normalized curve-space units. Output is always a Linear curve; closed rings keep their closed flag without repeating the seam point (the closing segment's length is still accounted for when computing spacing).".to_string(),
        }
    }

    /// Creates the default inputs: curve, mode, spacing, count.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to resample."),
            Input::new("mode".to_string(), Value::Text("spacing".to_string()), Some(InputSettings::Dropdown {
                options: vec!["spacing".to_string(), "count".to_string()],
            }), None)
                .with_description("spacing: resample at a fixed distance between points. count: resample to a target total point count."),
            Input::new("spacing".to_string(), Value::Decimal(8.0), Some(InputSettings::Slider { range: (1.0, 256.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Target distance between points, in pixels at a 1024px reference (divided by 1024 into normalized units). Used when mode = spacing."),
            Input::new("count".to_string(), Value::Integer(64), Some(InputSettings::DragValue { clamp: Some((2.0, 4000.0)), speed: None }), None)
                .with_description("Target total point count. Used when mode = count."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The resampled Linear curve."),
        ]
    }

    /// Resamples the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let mode_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let spacing_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let count_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Text(mode) = mode_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spacing) = spacing_converted.unwrap() else { unreachable!() };
        let Value::Integer(count) = count_converted.unwrap() else { unreachable!() };

        let spacing_norm = (spacing as f64).clamp(1.0, 256.0) / 1024.0;
        let count = count.clamp(2, MAX_OUTPUT_POINTS as i32) as usize;

        let out = resample_curve(&curve, &mode, spacing_norm, count);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
