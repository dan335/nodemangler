//! Two-argument arctangent (atan2) operation for the node graph.
//!
//! Computes atan2(y, x), returning the angle in radians between the positive
//! x-axis and the point (x, y).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes atan2(y, x).
///
/// Takes two inputs (y and x) and returns the angle in radians.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAtan2 {}

impl OpNumberTrigAtan2 {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "atan2".to_string(),
            description: "Computes atan2(y, x), the two-argument arctangent.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (y and x).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("y".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("x".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the atan2 operation.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let y_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };

        let result = y.atan2(x);

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
    async fn test_atan2_settings() {
        let s = OpNumberTrigAtan2::settings();
        assert_eq!(s.name, "atan2");
        assert_eq!(OpNumberTrigAtan2::create_inputs().len(), 2);
        assert_eq!(OpNumberTrigAtan2::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_atan2_y1_x0() {
        // atan2(1, 0) = pi/2
        let mut inputs = vec![
            Input::new("y".to_string(), Value::Decimal(1.0), None, None),
            Input::new("x".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_atan2_y0_x1() {
        // atan2(0, 1) = 0
        let mut inputs = vec![
            Input::new("y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("x".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(v.abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_atan2_yn1_x0() {
        // atan2(-1, 0) = -pi/2
        let mut inputs = vec![
            Input::new("y".to_string(), Value::Decimal(-1.0), None, None),
            Input::new("x".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-std::f32::consts::FRAC_PI_2)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_atan2_y1_x1() {
        // atan2(1, 1) = pi/4
        let mut inputs = vec![
            Input::new("y".to_string(), Value::Decimal(1.0), None, None),
            Input::new("x".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_4).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_atan2_from_integer() {
        let mut inputs = vec![
            Input::new("y".to_string(), Value::Integer(1), None, None),
            Input::new("x".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
