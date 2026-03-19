//! Logical NOR operation.
//!
//! Returns `true` only when both inputs are `false` (negation of OR). Inputs
//! are coerced to boolean before evaluation.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Logical NOR gate node.
///
/// Takes two boolean-convertible inputs and outputs `!(a || b)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicBoolNor {}

impl OpLogicBoolNor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "nor".to_string(),
            description: "Returns true only if both inputs are false.".to_string(),
        }
    }

    /// Creates the default inputs: two boolean inputs `a` and `b`, both defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Bool(false), None, None),
            Input::new("b".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `true` (NOR of two false values).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(true), None)
        ]
    }

    /// Converts both inputs to booleans and returns the negation of their disjunction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Bool(b) = b_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Bool(!(a || b)),
            }],
        })
    }
}

#[cfg(test)]
#[path = "nor_tests.rs"]
mod tests;
