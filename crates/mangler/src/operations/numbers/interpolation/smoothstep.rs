//! Smoothstep operation for the node graph.
//!
//! Performs smooth Hermite interpolation between two edge values. The result is
//! clamped to `[0, 1]` and follows the standard GLSL `smoothstep` formula:
//! `t * t * (3 - 2t)` where `t = clamp((x - edge0) / (edge1 - edge0), 0, 1)`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes smooth Hermite interpolation between two edges.
///
/// All inputs are converted to decimal. Returns an error if `edge0 == edge1`
/// (degenerate range). The output is always in the range `[0.0, 1.0]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathSmoothstep {}

impl OpNumberMathSmoothstep {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "smoothstep".to_string(),
            description: "Smooth Hermite interpolation between two edges.".to_string(),
        }
    }

    /// Creates the default input list: "input" (0.5), "edge0" (0.0), and "edge1" (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("edge0".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the smoothstep operation.
    ///
    /// Computes `t = clamp((input - edge0) / (edge1 - edge0), 0, 1)` then
    /// returns `t * t * (3 - 2t)`. Returns an error if `edge0 == edge1`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let edge0_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let edge1_val = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_val.unwrap() else { unreachable!() };
        let Value::Decimal(edge0) = edge0_val.unwrap() else { unreachable!() };
        let Value::Decimal(edge1) = edge1_val.unwrap() else { unreachable!() };

        // validate edges are different
        if edge0 == edge1 {
            return Err(OperationError {
                input_errors: vec![], node_error: Some("edge0 and edge1 must be different.".to_string()),
            });
        }

        // run node: smoothstep formula
        let t = ((input - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        let value = Value::Decimal(t * t * (3.0 - 2.0 * t));

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
    async fn test_smoothstep_settings() {
        let s = OpNumberMathSmoothstep::settings();
        assert_eq!(s.name, "smoothstep");
        assert_eq!(OpNumberMathSmoothstep::create_inputs().len(), 3);
        assert_eq!(OpNumberMathSmoothstep::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_smoothstep_midpoint() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.5), None, None),
            Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.5).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_smoothstep_at_edge0() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.0), None, None),
            Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_smoothstep_at_edge1() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(1.0), None, None),
            Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_smoothstep_quarter() {
        // smoothstep(0.25, 0, 1) = 0.25^2 * (3 - 2*0.25) = 0.0625 * 2.5 = 0.15625
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.25), None, None),
            Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.15625).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_smoothstep_equal_edges_errors() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.5), None, None),
            Input::new("edge0".to_string(), Value::Decimal(1.0), None, None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathSmoothstep::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for equal edges");
    }
}
