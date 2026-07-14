//! Lissajous curve generator.
//!
//! Classic `x = rx * sin(a*t + phi)`, `y = ry * sin(b*t)` parametric figure,
//! sampled densely over one period as a closed polyline.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::linear_curve;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "lissajous_tests.rs"]
mod tests;

/// Builds a closed Lissajous curve centered at `(cx, cy)`: `x = rx *
/// sin(freq_a * t + phase)`, `y = ry * sin(freq_b * t)`, sampled at `points`
/// steps over one period `t in [0, 2*pi)`.
pub(crate) fn lissajous_points(cx: f64, cy: f64, rx: f64, ry: f64, freq_a: f64, freq_b: f64, phase: f64, points: usize) -> Vec<[f32; 2]> {
    (0..points)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / points as f64;
            let x = rx * (freq_a * t + phase).sin();
            let y = ry * (freq_b * t).sin();
            [(cx + x) as f32, (cy + y) as f32]
        })
        .collect()
}

/// Operation that generates a closed Lissajous figure curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorLissajous {}

impl OpCurveGeneratorLissajous {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lissajous".to_string(),
            description: "Generates a closed Lissajous figure curve.".to_string(),
            help: "Builds the classic Lissajous parametric figure: x = radius_x * sin(freq_a * t + phase), y = radius_y * sin(freq_b * t), sampled densely as a closed Linear polyline over one period. Integer frequency ratios (e.g. 3:2) give the familiar closed figure-eight-like loops; non-integer ratios still close after one sampled period but trace a less symmetric path.\n\nAll positions and radii are normalized 0-1 curve-space units. Feed the output into rasterize curve, or into smooth for a Catmull-Rom pass over the same points.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, radius x/y, freq a/b, phase, points.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the figure in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the figure in normalized [0,1] curve space."),
            Input::new("radius x".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal amplitude in normalized units."),
            Input::new("radius y".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical amplitude in normalized units."),
            Input::new("freq a".to_string(), Value::Decimal(3.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.05) }), None)
                .with_description("Horizontal frequency. Integer a:b ratios give closed figures with a x b lobes."),
            Input::new("freq b".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.05) }), None)
                .with_description("Vertical frequency. Integer a:b ratios give closed figures with a x b lobes."),
            Input::new("phase".to_string(), Value::Decimal(90.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Phase offset of the horizontal term, in degrees."),
            Input::new("points".to_string(), Value::Integer(256), Some(InputSettings::DragValue { clamp: Some((64.0, 1024.0)), speed: None }), None)
                .with_description("Sample density over one period (higher = smoother, more points)."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated closed Lissajous curve."),
        ]
    }

    /// Generates the Lissajous curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let cx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let rx_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let ry_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let freq_a_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let freq_b_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let phase_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let points_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rx) = rx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ry) = ry_converted.unwrap() else { unreachable!() };
        let Value::Decimal(freq_a) = freq_a_converted.unwrap() else { unreachable!() };
        let Value::Decimal(freq_b) = freq_b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(phase) = phase_converted.unwrap() else { unreachable!() };
        let Value::Integer(points) = points_converted.unwrap() else { unreachable!() };

        let rx = (rx as f64).max(0.0);
        let ry = (ry as f64).max(0.0);
        let points = (points.clamp(64, 1024)) as usize;
        let phase_rad = (phase as f64).to_radians();

        let pts = lissajous_points(cx as f64, cy as f64, rx, ry, freq_a as f64, freq_b as f64, phase_rad, points);
        let curve = linear_curve(pts, true);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
