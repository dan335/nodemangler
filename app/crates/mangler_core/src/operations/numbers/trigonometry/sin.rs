//! Sine operation for the node graph.
//!
//! Computes the sine of an angle in radians.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the sine of a value in radians.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigSin {}

impl OpNumberTrigSin {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sin".to_string(),
            description: "Computes the sine of an angle in radians.".to_string(),
            help: "Returns sin(input) with the input interpreted as radians; the output always lies in [-1, 1].\n\nIf you are working in degrees, multiply by pi / 180 before feeding the node. Very large input magnitudes lose precision because radians collapse onto the same angle modulo tau. Pair with asin for the principal inverse.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Angle in radians to take the sine of."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Sine of the input angle, always in [-1, 1].")
        ]
    }

    /// Executes the sine operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.sin();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "sin_tests.rs"]
mod tests;
