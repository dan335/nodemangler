//! Superellipse (Lame curve) curve generator.
//!
//! Generalizes the ellipse via an exponent: 2 = ellipse, >2 rounds toward a
//! rectangle (squircle), <2 pinches toward a 4-pointed star/diamond. Sampled
//! densely since the shape isn't representable with a handful of Bezier
//! anchors once the exponent moves away from 2.

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
#[path = "superellipse_tests.rs"]
mod tests;

/// Builds the closed superellipse (Lame curve) polyline centered at
/// `(cx, cy)` with semi-axes `(rx, ry)` and shape `exponent` (2 = ellipse),
/// rotated `rotation_deg`, sampled at `samples` points around the parameter
/// circle.
pub(crate) fn superellipse_points(
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    exponent: f64,
    rotation_deg: f64,
    samples: usize,
) -> Vec<[f32; 2]> {
    let rot = rotation_deg.to_radians();
    let (rs, rc) = rot.sin_cos();
    let exp = 2.0 / exponent;
    (0..samples)
        .map(|i| {
            let theta = std::f64::consts::TAU * i as f64 / samples as f64;
            let (s, c) = theta.sin_cos();
            let lx = rx * c.signum() * c.abs().powf(exp);
            let ly = ry * s.signum() * s.abs().powf(exp);
            let x = lx * rc - ly * rs;
            let y = lx * rs + ly * rc;
            [(cx + x) as f32, (cy + y) as f32]
        })
        .collect()
}

/// Operation that generates a closed superellipse (Lame curve) curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorSuperellipse {}

impl OpCurveGeneratorSuperellipse {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "superellipse".to_string(),
            description: "Generates a closed superellipse (Lame curve) curve.".to_string(),
            help: "Builds a closed superellipse (Lame curve): |x/rx|^n + |y/ry|^n = 1, sampled densely as a Linear polyline. Exponent 2 is a plain ellipse; higher exponents round toward a rectangle (a 'squircle' near 4-5); lower exponents pinch toward a 4-pointed star/diamond (1 is a diamond, below 1 concave).\n\nAll positions and radii are normalized 0-1 curve-space units. Feed the output into rasterize curve, or into round corners for extra rounding on the squircle end.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, radius x/y, exponent, rotation, points.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the superellipse in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the superellipse in normalized [0,1] curve space."),
            Input::new("radius x".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal semi-axis in normalized units."),
            Input::new("radius y".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical semi-axis in normalized units."),
            Input::new("exponent".to_string(), Value::Decimal(2.5), Some(InputSettings::Slider { range: (0.2, 8.0), step_by: Some(0.05), clamp_to_range: false }), None)
                .with_description("Shape exponent: 2 = ellipse, higher rounds toward a rectangle, lower pinches toward a diamond/star."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation about the center, in degrees."),
            Input::new("points".to_string(), Value::Integer(128), Some(InputSettings::DragValue { clamp: Some((16.0, 512.0)), speed: None }), None)
                .with_description("Sample density around the curve (higher = smoother, more points)."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated closed superellipse curve."),
        ]
    }

    /// Generates the superellipse curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let cx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let rx_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let ry_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let exponent_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let points_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rx) = rx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ry) = ry_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exponent) = exponent_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };
        let Value::Integer(points) = points_converted.unwrap() else { unreachable!() };

        let rx = (rx as f64).max(0.001);
        let ry = (ry as f64).max(0.001);
        let exponent = (exponent as f64).clamp(0.2, 8.0);
        let points = (points.clamp(16, 512)) as usize;

        let pts = superellipse_points(cx as f64, cy as f64, rx, ry, exponent, rotation as f64, points);
        let curve = linear_curve(pts, true);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
