//! Cosine operation for the node graph.
//!
//! Computes the cosine of an angle in radians.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the cosine of a value in radians.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigCos {}

impl OpNumberTrigCos {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cos".to_string(),
            description: "Computes the cosine of an angle in radians.".to_string(),
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

    /// Executes the cosine operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.cos();

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
    async fn test_cos_settings() {
        let s = OpNumberTrigCos::settings();
        assert_eq!(s.name, "cos");
        assert_eq!(OpNumberTrigCos::create_inputs().len(), 1);
        assert_eq!(OpNumberTrigCos::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_cos_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cos_pi_over_2() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::FRAC_PI_2), None, None)];
        let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(v.abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cos_pi() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), None, None)];
        let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cos_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
