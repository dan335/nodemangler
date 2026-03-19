//! Cast-to-integer operation for the node graph.
//!
//! Converts a numeric value to an integer (i32) using `try_convert_to`.
//! Decimal inputs are truncated toward zero; integer inputs pass through unchanged.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to integer (i32).
///
/// Uses `Value::try_convert_to(ValueType::Integer)` for the conversion.
/// Decimal values are truncated toward zero (not rounded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCastToInteger {}

impl OpNumberCastToInteger {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to integer".to_string(),
            description: "Converts a number to an integer.".to_string(),
        }
    }

    /// Creates the default input list: a single integer drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(i32::default()), None)
        ]
    }

    /// Executes the cast: converts the input to an integer via `try_convert_to`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Integer(n)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to integer.".to_string())], node_error: None })};

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(n),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_integer_tests.rs"]
mod tests;
