//! Power (exponentiation) operation for the node graph.
//!
//! Computes `base^exponent` using f64 intermediate precision to reduce rounding
//! errors, then casts the result back to f32.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that raises a base to an exponent.
///
/// Both inputs are converted to decimal. The computation uses f64 precision
/// internally (`(base as f64).powf(exponent as f64)`) before casting to f32.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathPow {}

impl OpNumberMathPow {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "power".to_string(),
            description: "Raises base to an exponent.".to_string(),
        }
    }

    /// Creates the default input list: `base` (2.0) and `exponent` (2.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("base".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("exponent".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the power operation: computes `base^exponent`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let base_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let exponent_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(base) = base_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exponent) = exponent_converted.unwrap() else { unreachable!() };

        let result = (base as f64).powf(exponent as f64) as f32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
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
    async fn test_pow_settings() {
        let s = OpNumberMathPow::settings();
        assert_eq!(s.name, "power");
        assert_eq!(OpNumberMathPow::create_inputs().len(), 2);
        assert_eq!(OpNumberMathPow::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_pow_basic() {
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(2.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 8.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_zero_exponent() {
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(5.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_fractional() {
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(4.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_zero_base() {
        // 0^n = 0 for n > 0
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(0.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_zero_zero() {
        // 0^0 = 1 in Rust's powf
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(0.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_negative_exponent() {
        // 2^(-2) = 0.25
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(2.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(-2.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.25).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_base_one() {
        // 1^n = 1 for any n
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(1.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(1000.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_negative_base() {
        // (-2)^3 = -8
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Decimal(-2.0), None, None),
            Input::new("exponent".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-8.0)).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pow_from_integer() {
        let mut inputs = vec![
            Input::new("base".to_string(), Value::Integer(3), None, None),
            Input::new("exponent".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 81.0).abs() < 1e-3),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
