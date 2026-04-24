//! Tangent operation for the node graph.
//!
//! Computes the tangent of an angle in radians.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the tangent of a value in radians.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigTan {}

impl OpNumberTrigTan {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tan".to_string(),
            description: "Computes the tangent of an angle in radians.".to_string(),
            help: "Returns sin(x) / cos(x) with the input interpreted as radians. The function has vertical asymptotes at pi/2 + k*pi, where cos goes to zero, so inputs near those angles return very large magnitudes (positive or negative).\n\nIf you have y and x components and want a quadrant-correct angle, use atan2. Pair with atan for the principal inverse on (-pi/2, pi/2).".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Angle in radians to take the tangent of."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Tangent of the input angle; diverges near pi/2 + k*pi.")
        ]
    }

    /// Executes the tangent operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.tan();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "tan_tests.rs"]
mod tests;
