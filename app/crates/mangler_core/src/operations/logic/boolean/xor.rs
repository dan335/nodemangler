//! Logical XOR (exclusive OR) operation.
//!
//! Returns `true` when exactly one of the two inputs is `true`. Inputs are
//! coerced to boolean before evaluation.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Logical XOR gate node.
///
/// Takes two boolean-convertible inputs and outputs `a ^ b`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicBoolXor {}

impl OpLogicBoolXor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "xor".to_string(),
            description: "Returns true if exactly one input is true.".to_string(),
        }
    }

    /// Creates the default inputs: two boolean inputs `a` and `b`, both defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Bool(false), None, None),
            Input::new("b".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
        ]
    }

    /// Converts both inputs to booleans and returns their exclusive disjunction.
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
                value: Value::Bool(a ^ b),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn make_inputs(a: Value, b: Value) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), a, None, None),
            Input::new("b".to_string(), b, None, None),
        ]
    }

    #[tokio::test]
    async fn test_xor_true_false() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Bool(false));
        let result = OpLogicBoolXor::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_xor_true_true() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
        let result = OpLogicBoolXor::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_xor_false_false() {
        let mut inputs = make_inputs(Value::Bool(false), Value::Bool(false));
        let result = OpLogicBoolXor::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_xor_false_true() {
        let mut inputs = make_inputs(Value::Bool(false), Value::Bool(true));
        let result = OpLogicBoolXor::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_xor_settings() {
        let s = OpLogicBoolXor::settings();
        assert_eq!(s.name, "xor");
    }
}
