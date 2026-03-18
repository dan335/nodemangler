//! Modulus (remainder) operation for the node graph.
//!
//! Computes `a % n` for integer and decimal types. Returns an error if `n` is zero.
//! Uses Rust's remainder semantics (result has the sign of the dividend).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the remainder of `a` divided by `n`.
///
/// The divisor `n` is converted to decimal for validation. Returns an error
/// when `n` is zero. The result sign matches the dividend (Rust `%` semantics).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathModulus {}

impl OpNumberMathModulus {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "modulus".to_string(),
            description: "Divides two numbers and returns the remainder.".to_string(),
        }
    }

    /// Creates the default input list: value `a` (0.5) and divisor `n` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("n".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the modulus: computes `a % n`, returning an error if `n` is zero.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(n)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert 'n' to Decimal.".to_string())], node_error: None })};

        if n == 0.0 {
            return Err(OperationError {
                input_errors: vec![(1, "Division by zero.".to_string())], node_error: None,
            });
        }

        let value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer(*a % n as i32),

            Value::Decimal(a) => Value::Decimal(*a % n as f32),

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
    async fn test_modulus_settings() {
        let s = OpNumberMathModulus::settings();
        assert_eq!(s.name, "modulus");
        assert_eq!(OpNumberMathModulus::create_inputs().len(), 2);
        assert_eq!(OpNumberMathModulus::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_modulus_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(10.0), None, None),
            Input::new("b".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_modulus_by_zero_errors() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(10.0), None, None),
            Input::new("n".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for mod by zero");
    }

    #[tokio::test]
    async fn test_modulus_integer_by_zero_errors() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(10), None, None),
            Input::new("n".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for integer mod by zero");
    }

    #[tokio::test]
    async fn test_modulus_integer_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(10), None, None),
            Input::new("n".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_modulus_exact_divisible() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(12), None, None),
            Input::new("n".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_modulus_negative_dividend() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-10), None, None),
            Input::new("n".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            // Rust % operator returns -1 for -10 % 3
            Value::Integer(v) => assert_eq!(*v, -1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_modulus_decimal_fractional() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(5.5), None, None),
            Input::new("n".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.5).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_modulus_invalid_type_returns_error() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Bool(true), None, None),
            Input::new("n".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathModulus::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for unsupported type");
    }
}
