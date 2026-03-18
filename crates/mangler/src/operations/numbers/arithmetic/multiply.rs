//! Multiplication operation for the node graph.
//!
//! Computes `a * b` for integer and decimal types, with automatic type promotion
//! when mixing integers and decimals.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that multiplies two numbers together.
///
/// Supports integer and decimal types. Mixed types promote to decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathMultiply {}

impl OpNumberMathMultiply {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "multiply".to_string(),
            description: "Multiplies two numbers.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (a and b), defaulting to 1.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("b".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the multiplication: computes `a * b`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 * *b),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a * *b),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(*a * *b),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(*a * *b as f32),

            _ => {return Err(OperationError {
                input_errors: vec![], node_error: Some("Error converting.".to_string()),
            });}
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: value,
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
    async fn test_multiply_settings() {
        let s = OpNumberMathMultiply::settings();
        assert_eq!(s.name, "multiply");
        assert_eq!(OpNumberMathMultiply::create_inputs().len(), 2);
        assert_eq!(OpNumberMathMultiply::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_multiply_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(4.0), None, None),
            Input::new("b".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 20.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_by_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(100.0), None, None),
            Input::new("b".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_integer_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(6), None, None),
            Input::new("b".to_string(), Value::Integer(7), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_integer_decimal() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(3), None, None),
            Input::new("b".to_string(), Value::Decimal(1.5), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.5).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_decimal_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(2.5), None, None),
            Input::new("b".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_negatives() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-5), None, None),
            Input::new("b".to_string(), Value::Integer(-3), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 15),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_negative_positive() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-5), None, None),
            Input::new("b".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -15),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_by_one() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(42.5), None, None),
            Input::new("b".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 42.5).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_tiny_decimals() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(0.001), None, None),
            Input::new("b".to_string(), Value::Decimal(0.001), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1e-6).abs() < 1e-8),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_multiply_invalid_type_returns_error() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Bool(true), None, None),
            Input::new("b".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberMathMultiply::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for unsupported type");
    }
}
