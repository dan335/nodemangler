//! Reciprocal operation for the node graph.
//!
//! Computes `1/x` for a given number. Returns an error when the input is zero.
//! The input is converted to decimal before computation.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the reciprocal (1/x) of a number.
///
/// The input is converted to decimal. Returns an error when the input is zero
/// to prevent division by zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathReciprocal {}

impl OpNumberMathReciprocal {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "reciprocal".to_string(),
            description: "Returns 1 divided by the input value.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input defaulting to 1.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the reciprocal operation: computes `1.0 / input`.
    ///
    /// Returns an error if the input is zero (division by zero).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_val.unwrap() else { unreachable!() };

        // validate input is not zero
        if input == 0.0 {
            return Err(OperationError {
                input_errors: vec![], node_error: Some("Division by zero.".to_string()),
            });
        }

        // run node
        let value = Value::Decimal(1.0 / input);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "reciprocal_tests.rs"]
mod tests;
