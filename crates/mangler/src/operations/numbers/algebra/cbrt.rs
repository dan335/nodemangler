//! Cube root operation for the node graph.
//!
//! Computes the cube root of a number. Unlike square root, cube root handles
//! negative inputs (returning a negative result).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the cube root of a number.
///
/// Always returns a decimal. Integer inputs are cast to f32 before computing.
/// Negative inputs produce negative results (e.g., cbrt(-8) = -2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathCbrt {}

impl OpNumberMathCbrt {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cube root".to_string(),
            description: "Returns the cube root of a number.".to_string(),
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

    /// Executes the cube root: computes `cbrt(a)`.
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

            Value::Integer(a) => Value::Decimal((*a as f32).cbrt()),
            Value::Decimal(a) => Value::Decimal(a.cbrt()),

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
    async fn test_cbrt_settings() {
        let s = OpNumberMathCbrt::settings();
        assert_eq!(s.name, "cube root");
        assert_eq!(OpNumberMathCbrt::create_inputs().len(), 1);
        assert_eq!(OpNumberMathCbrt::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_cbrt_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(27.0), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cbrt_of_zero() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cbrt_of_one() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cbrt_of_negative() {
        // f32::cbrt handles negative numbers (returns negative root)
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-8.0), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-2.0)).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cbrt_of_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(8), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.0).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cbrt_non_perfect_cube() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(2.0), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            // cbrt(2) ≈ 1.2599
            Value::Decimal(v) => assert!((*v - 1.2599).abs() < 0.001),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cbrt_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
        let result = OpNumberMathCbrt::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
