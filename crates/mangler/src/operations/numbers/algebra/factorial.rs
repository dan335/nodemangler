//! Factorial operation for the node graph.
//!
//! Computes `n!` for non-negative integers. The input is clamped to `[0, 12]`
//! because `12! = 479,001,600` is the largest factorial that fits in an `i32`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the factorial of an integer.
///
/// Input is clamped to `[0, 12]` to prevent i32 overflow. Decimal inputs are
/// first converted (truncated) to integer via `convert_input`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathFactorial {}

impl OpNumberMathFactorial {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "factorial".to_string(),
            description: "Computes the factorial of an integer.".to_string(),
        }
    }

    /// Creates the default input list: a single integer input clamped to `[0, 12]`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(5), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 12.0)) }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the factorial: computes `n!` with input clamped to `[0, 12]`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(val) = input_converted.unwrap() else { unreachable!() };

        let val = val.clamp(0, 12); // 12! = 479001600, max that fits in i32

        let mut result: i32 = 1;
        for i in 2..=(val) {
            result *= i;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(result),
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
    async fn test_factorial_settings() {
        let s = OpNumberMathFactorial::settings();
        assert_eq!(s.name, "factorial");
        assert_eq!(OpNumberMathFactorial::create_inputs().len(), 1);
        assert_eq!(OpNumberMathFactorial::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_factorial_5() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(5), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 120),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_0() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_1() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_12() {
        // 12! = 479001600, the max that fits in i32
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(12), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 479001600),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_large_input_clamped_to_12() {
        // Input > 12 is clamped to 12
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(100), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 479001600),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_negative_input_clamped_to_zero() {
        // Negative input is clamped to 0, so result is 0! = 1
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(-5), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_2() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(2), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 2),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_factorial_from_decimal() {
        // Decimal input is converted to Integer via convert_input (truncated)
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.9), None, None)];
        let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            // 5! = 120
            Value::Integer(v) => assert_eq!(*v, 120),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
