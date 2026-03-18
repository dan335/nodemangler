//! Square root operation for the node graph.
//!
//! Computes the square root of a non-negative number. Returns an error for
//! negative inputs (unlike cube root, which handles negatives).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the square root of a number.
///
/// Always returns a decimal. Returns an error if the input is negative.
/// Integer inputs are cast to f32 before computing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathSqrt {}

impl OpNumberMathSqrt {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "square root".to_string(),
            description: "Returns the square root of a number.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the square root, returning an error for negative inputs.
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

            Value::Integer(a) if *a < 0 => {
                return Err(OperationError {
                    input_errors: vec![(0, "Cannot take square root of a negative number.".to_string())], node_error: None,
                });
            },
            Value::Decimal(a) if *a < 0.0 => {
                return Err(OperationError {
                    input_errors: vec![(0, "Cannot take square root of a negative number.".to_string())], node_error: None,
                });
            },
            Value::Integer(a) => Value::Decimal((*a as f32).sqrt()),
            Value::Decimal(a) => Value::Decimal(a.sqrt()),

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
    async fn test_sqrt_settings() {
        let s = OpNumberMathSqrt::settings();
        assert_eq!(s.name, "square root");
        assert_eq!(OpNumberMathSqrt::create_inputs().len(), 1);
        assert_eq!(OpNumberMathSqrt::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_sqrt_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(9.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_negative_decimal_errors() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-1.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for sqrt of negative decimal");
    }

    #[tokio::test]
    async fn test_sqrt_negative_integer_errors() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-4), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for sqrt of negative integer");
    }

    #[tokio::test]
    async fn test_sqrt_of_one() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_integer_input() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(16), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_non_perfect_square() {
        // sqrt(2) ≈ 1.41421
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(2.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.41421).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_large_number() {
        // sqrt(1000000) = 1000
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(1000000.0), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1000.0).abs() < 0.1),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sqrt_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
        let result = OpNumberMathSqrt::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
