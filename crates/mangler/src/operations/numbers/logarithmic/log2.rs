//! Base-2 logarithm operation for the node graph.
//!
//! Computes log base 2 of the input. Returns an error if input is not positive.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the base-2 logarithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLog2 {}

impl OpNumberMathLog2 {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "log2".to_string(),
            description: "Computes base-2 logarithm.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
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

    /// Executes the log2 operation, validating that input is positive.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        if input <= 0.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Input must be greater than 0.".to_string()) });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(input.log2()),
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
    async fn test_log2_settings() {
        let s = OpNumberMathLog2::settings();
        assert_eq!(s.name, "log2");
        assert_eq!(OpNumberMathLog2::create_inputs().len(), 1);
        assert_eq!(OpNumberMathLog2::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_log2_of_8() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(8.0), None, None)];
        let result = OpNumberMathLog2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_log2_of_1() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathLog2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(v.abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_log2_zero_errors() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathLog2::run(&mut inputs).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_log2_negative_errors() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
        let result = OpNumberMathLog2::run(&mut inputs).await;
        assert!(result.is_err());
    }
}
