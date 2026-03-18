//! Sign operation for the node graph.
//!
//! Returns the sign of a number: -1, 0, or 1 for integers; -1.0 or 1.0 for
//! decimals (note: `f32::signum(0.0)` returns `1.0`, not `0.0`).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that returns the sign of a number.
///
/// Uses `i32::signum` for integers (returns -1, 0, or 1) and `f32::signum`
/// for decimals (returns -1.0 or 1.0; positive zero returns 1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathSign {}

impl OpNumberMathSign {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sign".to_string(),
            description: "Returns the sign of a number.".to_string(),
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

    /// Executes the sign operation: returns the signum of the input.
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
            Value::Integer(a)=> Value::Integer((*a).signum()),
            Value::Decimal(a)=> Value::Decimal((*a).signum()),

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
    async fn test_sign_settings() {
        let s = OpNumberMathSign::settings();
        assert_eq!(s.name, "sign");
        assert_eq!(OpNumberMathSign::create_inputs().len(), 1);
        assert_eq!(OpNumberMathSign::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_sign_positive() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-5.0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_zero() {
        // f32::signum(0.0) == 1.0 in Rust (positive zero returns 1.0)
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_integer_positive() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_integer_negative() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-42), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_integer_zero() {
        // i32::signum(0) == 0
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_small_positive_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0001), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_small_negative_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.0001), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sign_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(false), None, None)];
        let result = OpNumberMathSign::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
