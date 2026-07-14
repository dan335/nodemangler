//! Curve point sampling.
//!
//! Reads off a curve's position and tangent direction at a normalized
//! arc-length parameter, so a single slider can walk a point (and a heading)
//! along any curve — driving a spawn position, a camera, or anything else
//! that needs to travel a path.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that samples a curve's position and tangent angle at a given `t`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCurveSamplePoint {}

impl OpNumberCurveSamplePoint {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sample point".to_string(),
            description: "Reads a curve's position and tangent angle at a normalized arc-length parameter.".to_string(),
            help: "Samples the curve at normalized arc-length parameter `t` (0 = curve start, 1 = curve end) and reports its position (x, y, in [0,1]² units) alongside the tangent direction there as `angle` in degrees.\n\nAngle uses screen convention: 0° points along +x, and since y is already down, positive angles rotate clockwise on screen. An empty curve (no points) reports (0, 0, 0°).".to_string(),
        }
    }

    /// Creates the inputs: curve and the arc-length parameter t.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve to sample."),
            Input::new("t".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Normalized arc-length parameter (0 = start, 1 = end)."),
        ]
    }

    /// Creates the output ports: x, y, angle.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.0), None)
                .with_description("Sampled x position in normalized [0,1] units."),
            Output::new("y".to_string(), Value::Decimal(0.0), None)
                .with_description("Sampled y position in normalized [0,1] units."),
            Output::new("angle".to_string(), Value::Decimal(0.0), None)
                .with_description("Tangent direction in degrees (0° = +x, positive = clockwise on screen)."),
        ]
    }

    /// Executes the sample.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let t_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t) = t_converted.unwrap() else { unreachable!() };

        let [x, y] = curve.sample(t);
        let [tx, ty] = curve.tangent_at(t);
        // y is already down, so this is the screen-space angle directly:
        // 0° = +x, positive = clockwise on screen.
        let angle = ty.atan2(tx).to_degrees();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(x) },
                OutputResponse { value: Value::Decimal(y) },
                OutputResponse { value: Value::Decimal(angle) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "sample_point_tests.rs"]
mod tests;
