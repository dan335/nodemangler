//! Two-argument arctangent (atan2) operation for the node graph.
//!
//! Computes atan2(y, x), returning the angle in radians between the positive
//! x-axis and the point (x, y).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes atan2(y, x).
///
/// Takes two inputs (y and x) and returns the angle in radians.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAtan2 {}

impl OpNumberTrigAtan2 {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "atan2".to_string(),
            description: "Computes atan2(y, x), the two-argument arctangent.".to_string(),
            help: "Returns the signed angle in radians between the positive x-axis and the point (x, y), in the range (-pi, pi]. Unlike plain atan, it uses the signs of both inputs to place the result in the correct quadrant.\n\nat y = 0, x = 0 the angle is defined as 0. Handy for converting cartesian coordinates into polar angles, e.g. when generating radial patterns.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (y and x).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("y".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Y coordinate (vertical component) of the point."),
            Input::new("x".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("X coordinate (horizontal component) of the point."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Angle in radians from the positive x-axis to (x, y), in (-pi, pi].")
        ]
    }

    /// Executes the atan2 operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let y_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };

        let result = y.atan2(x);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "atan2_tests.rs"]
mod tests;
