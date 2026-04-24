//! Logical NAND operation.
//!
//! Returns `true` unless both inputs are `true` (negation of AND). Inputs are
//! coerced to boolean before evaluation.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Logical NAND gate node.
///
/// Takes two boolean-convertible inputs and outputs `!(a && b)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicBoolNand {}

impl OpLogicBoolNand {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "nand".to_string(),
            description: "Returns true unless both inputs are true.".to_string(),
            help: "Negation of AND: outputs false only when both operands are true, and true in every other case. Truth table: (false, false) -> true, (false, true) -> true, (true, false) -> true, (true, true) -> false.\n\nInputs are coerced to Bool before evaluation. NAND is functionally complete, so any boolean circuit can be built from chained NAND gates alone.".to_string(),
        }
    }

    /// Creates the default inputs: two boolean inputs `a` and `b`, both defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Bool(false), None, None)
                .with_description("First boolean operand of the NAND gate."),
            Input::new("b".to_string(), Value::Bool(false), None, None)
                .with_description("Second boolean operand of the NAND gate."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `true` (NAND of two false values).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(true), None)
                .with_description("False only when both a and b are true.")
        ]
    }

    /// Converts both inputs to booleans and returns the negation of their conjunction.
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
                value: Value::Bool(!(a && b)),
            }],
        })
    }
}

#[cfg(test)]
#[path = "nand_tests.rs"]
mod tests;
