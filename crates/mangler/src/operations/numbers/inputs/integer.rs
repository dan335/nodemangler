//! Integer input node operation.
//!
//! Provides a single integer value to the graph. Accepts integer or decimal inputs
//! (decimals are truncated to integers via type conversion).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_integer_input_passthrough() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer(42), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(-100), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -100),
            other => panic!("Expected Integer(-100), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer(0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_from_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.7), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer(5), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_settings() {
        let s = OpNumberInputInteger::settings();
        assert_eq!(s.name, "integer");
        assert_eq!(OpNumberInputInteger::create_inputs().len(), 1);
        assert_eq!(OpNumberInputInteger::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_integer_input_max() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(i32::MAX), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, i32::MAX),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_min() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(i32::MIN), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, i32::MIN),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_large_positive() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1_000_000), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1_000_000),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_from_negative_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-7.9), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            // Decimal to integer truncates toward zero
            Value::Integer(v) => assert_eq!(*v, -7),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_integer_input_output_count() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpNumberInputInteger::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
    }
}

/// Node operation that outputs an integer value.
///
/// Passes through a single integer input as the output. Input values of other
/// numeric types are converted to integers (e.g., decimals are truncated).
#[derive(Clone, Serialize, Deserialize)]
pub struct OpNumberInputInteger {}


impl OpNumberInputInteger {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "integer".to_string(),
            description: "An integer number input.".to_string(),
        }
    }

    /// Creates the default input list: a single integer drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(i32::default()), None)
        ]
    }

    /// Executes the node: converts the input to an integer and passes it through.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(input) = input_converted.unwrap() else { unreachable!() };

        // run node
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(input),
            }],
        })
    }
}
