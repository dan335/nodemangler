//! Arccosine operation for the node graph.
//!
//! Computes the arccosine (inverse cosine) of a value. Input must be in [-1, 1].

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the arccosine (inverse cosine) of a value.
///
/// Input must be in the range [-1, 1]. Returns an error for out-of-range inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAcos {}

impl OpNumberTrigAcos {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "acos".to_string(),
            description: "Computes the arccosine (inverse cosine). Input must be in [-1, 1].".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the arccosine operation. Returns an error if input is outside [-1, 1].
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        // Validate input range for arccosine.
        if input < -1.0 || input > 1.0 {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some(format!("acos input must be in [-1, 1], got {}", input)),
            });
        }

        let result = input.acos();

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
    async fn test_acos_settings() {
        let s = OpNumberTrigAcos::settings();
        assert_eq!(s.name, "acos");
        assert_eq!(OpNumberTrigAcos::create_inputs().len(), 1);
        assert_eq!(OpNumberTrigAcos::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_acos_one() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(v.abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_acos_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_acos_negative_one() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
        let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::PI).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_acos_out_of_range() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
        let result = OpNumberTrigAcos::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for acos(2.0)");
        let err = result.unwrap_err();
        assert!(err.node_error.is_some());
    }

    #[tokio::test]
    async fn test_acos_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(v.abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
