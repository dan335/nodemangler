//! Negate operation for the node graph.
//!
//! Flips the sign of a number: positive becomes negative, negative becomes
//! positive. Works with both integer and decimal types.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that negates a number (flips its sign).
///
/// Supports integer and decimal types. Integer inputs produce integer outputs,
/// decimal inputs produce decimal outputs. Returns an error for unsupported types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathNegate {}

impl OpNumberMathNegate {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "negate".to_string(),
            description: "Negates a number (flips sign).".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the negate operation: returns the negation of the input value.
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
            Value::Integer(a) => Value::Integer(-a),
            Value::Decimal(a) => Value::Decimal(-a),

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
    async fn test_negate_settings() {
        let s = OpNumberMathNegate::settings();
        assert_eq!(s.name, "negate");
        assert_eq!(OpNumberMathNegate::create_inputs().len(), 1);
        assert_eq!(OpNumberMathNegate::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_negate_positive() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-5.0)).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_negate_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-3.0), None, None)];
        let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_negate_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_negate_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(7), None, None)];
        let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -7),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_negate_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
        let result = OpNumberMathNegate::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
