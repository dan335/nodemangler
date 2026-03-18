//! Rounding operation for the node graph.
//!
//! Rounds a decimal to the nearest whole number using "round half away from zero"
//! semantics (Rust's `f32::round`). Integers pass through unchanged.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that rounds a number to the nearest integer.
///
/// Decimals are rounded using `f32::round` (half away from zero). Integer
/// inputs pass through unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathRound {}

impl OpNumberMathRound {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "round".to_string(),
            description: "Rounds a number.".to_string(),
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

    /// Executes the round operation on the input value.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match &inputs[0].value {

            Value::Integer(a) => Value::Integer(*a),
            Value::Decimal(a)=> Value::Decimal(a.round()),

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
    async fn test_round_settings() {
        let s = OpNumberMathRound::settings();
        assert_eq!(s.name, "round");
        assert_eq!(OpNumberMathRound::create_inputs().len(), 1);
        assert_eq!(OpNumberMathRound::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_round_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.7), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_down() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.2), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_half_positive() {
        // f32::round rounds 0.5 to 1.0 (round half away from zero)
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.5), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_half_negative() {
        // f32::round rounds -0.5 to -1.0 (round half away from zero)
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.5), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_already_integer_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(4.0), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_integer_passthrough() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(7), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 7),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_zero() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_negative_value() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-3.7), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-4.0)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_round_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
        let result = OpNumberMathRound::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
