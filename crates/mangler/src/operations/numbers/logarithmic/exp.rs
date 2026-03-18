//! Exponential function (`e^x`) operation for the node graph.
//!
//! Computes Euler's number raised to the given power. Uses f64 intermediate
//! precision for accuracy, then casts back to f32.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes `e^x` (the exponential function).
///
/// Input is converted to decimal. Uses f64 for the computation to maintain
/// precision, then casts back to f32.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathExp {}

impl OpNumberMathExp {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "exp".to_string(),
            description: "Computes e raised to a power.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input defaulting to 1.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the exponential: computes `e^input`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = (input as f64).exp() as f32;

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
    async fn test_exp_settings() {
        let s = OpNumberMathExp::settings();
        assert_eq!(s.name, "exp");
        assert_eq!(OpNumberMathExp::create_inputs().len(), 1);
        assert_eq!(OpNumberMathExp::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_exp_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_one() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::E).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_negative() {
        // exp(-1) ≈ 0.3679
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.36788).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_two() {
        // exp(2) ≈ 7.389
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 7.389).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_large_positive() {
        // exp(20) is a large but finite number
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(20.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v > 0.0 && v.is_finite()),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_exp_large_negative() {
        // exp(-20) approaches 0
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-20.0), None, None)];
        let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v > 0.0 && *v < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
