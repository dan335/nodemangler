//! Logical AND operation.
//!
//! Returns `true` only when both inputs are `true`. Inputs are coerced to
//! boolean before evaluation (non-zero values are truthy).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Logical AND gate node.
///
/// Takes two boolean-convertible inputs and outputs `a && b`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicBoolAnd {}

impl OpLogicBoolAnd {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "and".to_string(),
            description: "Returns true if both inputs are true.".to_string(),
            help: "Standard two-input logical conjunction. Truth table: (false, false) -> false, (false, true) -> false, (true, false) -> false, (true, true) -> true.\n\nInputs are coerced to Bool before evaluation, so non-boolean sources (such as integers or decimals) use their truthy/zero interpretation. If an input cannot be converted, the node reports an input error rather than defaulting silently.".to_string(),
        }
    }

    /// Creates the default inputs: two boolean inputs `a` and `b`, both defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Bool(false), None, None)
                .with_description("First boolean operand of the AND gate."),
            Input::new("b".to_string(), Value::Bool(false), None, None)
                .with_description("Second boolean operand of the AND gate."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True when both a and b are true.")
        ]
    }

    /// Converts both inputs to booleans and returns their logical conjunction.
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
                value: Value::Bool(a && b),
            }],
        })
    }
}

#[cfg(test)]
#[path = "and_tests.rs"]
mod tests;
