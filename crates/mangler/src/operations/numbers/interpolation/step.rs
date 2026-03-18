//! Step operation for the node graph.
//!
//! Returns 0.0 if the input is less than the edge value, or 1.0 otherwise.
//! This is the GLSL-style step function.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that implements the step function.
///
/// Both inputs are converted to decimal. Returns `0.0` when `input < edge`,
/// and `1.0` otherwise (i.e., when `input >= edge`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathStep {}

impl OpNumberMathStep {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "step".to_string(),
            description: "Returns 0 if input < edge, 1 otherwise.".to_string(),
        }
    }

    /// Creates the default input list: "input" (0.0) and "edge" (0.5) drag-value inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("edge".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the step operation: returns `0.0` if `input < edge`, `1.0` otherwise.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let edge_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_val.unwrap() else { unreachable!() };
        let Value::Decimal(edge) = edge_val.unwrap() else { unreachable!() };

        // run node
        let value = Value::Decimal(if input < edge { 0.0 } else { 1.0 });

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
    async fn test_step_settings() {
        let s = OpNumberMathStep::settings();
        assert_eq!(s.name, "step");
        assert_eq!(OpNumberMathStep::create_inputs().len(), 2);
        assert_eq!(OpNumberMathStep::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_step_below_edge() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.3), None, None),
            Input::new("edge".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_step_at_edge() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.5), None, None),
            Input::new("edge".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_step_above_edge() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.7), None, None),
            Input::new("edge".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_step_edge_zero() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(1.0), None, None),
            Input::new("edge".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
