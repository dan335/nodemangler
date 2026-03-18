//! Bitwise OR operation for the node graph.
//!
//! Computes the bitwise OR of two integers.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the bitwise OR of two integers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberBitwiseOr {}

impl OpNumberBitwiseOr {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bitwise or".to_string(),
            description: "Computes the bitwise OR of two integers.".to_string(),
        }
    }

    /// Creates the default input list: two integer drag-value inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the bitwise OR operation.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Integer(b) = b_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(a | b),
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
    async fn test_bitwise_or_settings() {
        let s = OpNumberBitwiseOr::settings();
        assert_eq!(s.name, "bitwise or");
        assert_eq!(OpNumberBitwiseOr::create_inputs().len(), 2);
        assert_eq!(OpNumberBitwiseOr::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_bitwise_or_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(0b1100), None, None),
            Input::new("b".to_string(), Value::Integer(0b1010), None, None),
        ];
        let result = OpNumberBitwiseOr::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0b1110),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bitwise_or_with_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(0), None, None),
            Input::new("b".to_string(), Value::Integer(0xFF), None, None),
        ];
        let result = OpNumberBitwiseOr::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0xFF),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bitwise_or_same_value() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(0b1010), None, None),
            Input::new("b".to_string(), Value::Integer(0b1010), None, None),
        ];
        let result = OpNumberBitwiseOr::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0b1010),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
