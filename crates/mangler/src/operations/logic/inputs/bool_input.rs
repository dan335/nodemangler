//! Boolean input operation.
//!
//! Provides a simple pass-through node that accepts a boolean value (or a value
//! convertible to boolean) and outputs it. Useful as an entry point for boolean
//! data in the node graph.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A boolean input node that passes through a boolean value.
///
/// Accepts any value convertible to `Bool` (e.g., integers where 0 is false,
/// non-zero is true) and outputs the converted boolean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicInputBool {}

impl OpLogicInputBool {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bool".to_string(),
            description: "A boolean input.".to_string(),
        }
    }

    /// Creates the default inputs: a single boolean input defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Bool(false), None, None)
        ]
    }

    /// Creates the default outputs: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
        ]
    }

    /// Converts the input to a boolean and passes it through as the output.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(input) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Bool(input),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_bool_input_true() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, true),
            other => panic!("Expected Bool(true), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_input_false() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, false),
            other => panic!("Expected Bool(false), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_input_from_integer_nonzero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, true),
            other => panic!("Expected Bool(true), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_input_from_integer_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, false),
            other => panic!("Expected Bool(false), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_settings() {
        let s = OpLogicInputBool::settings();
        assert_eq!(s.name, "bool");
        assert_eq!(OpLogicInputBool::create_inputs().len(), 1);
        assert_eq!(OpLogicInputBool::create_outputs().len(), 1);
    }
}
