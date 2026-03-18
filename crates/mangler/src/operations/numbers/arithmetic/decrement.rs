//! Decrement operation for the node graph.
//!
//! Subtracts 1 from an integer or decimal value. For strings, appends " -1".

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that decrements a value by 1.
///
/// For integers, subtracts 1. For decimals, subtracts 1.0. For strings, appends " -1".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathDecrement {}

impl OpNumberMathDecrement {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "decrement".to_string(),
            description: "Decrements a number by 1.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the decrement: subtracts 1 from the input value.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer(*a - 1),

            Value::Decimal(a) => Value::Decimal(*a - 1.0),

            Value::Text(a) => Value::Text(format!("{} {}", *a, -1)),

            _ => {return Err(OperationError {
                input_errors: vec![], node_error: Some("Error converting.".to_string()),
            });}
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
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
    async fn test_decrement_settings() {
        let s = OpNumberMathDecrement::settings();
        assert_eq!(s.name, "decrement");
        assert_eq!(OpNumberMathDecrement::create_inputs().len(), 1);
        assert_eq!(OpNumberMathDecrement::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_decrement_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decrement_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(10), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 9),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decrement_zero() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decrement_negative() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-5), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -6),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decrement_negative_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.5), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-1.5)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decrement_text() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Text("hello".to_string()), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(s) => assert_eq!(s, "hello -1"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decrement_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(false), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }

    #[tokio::test]
    async fn test_decrement_large_negative_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-(i32::MAX / 2)), None, None)];
        let result = OpNumberMathDecrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -(i32::MAX / 2) - 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
