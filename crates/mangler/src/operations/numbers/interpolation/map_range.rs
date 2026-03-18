//! Map range operation for the node graph.
//!
//! Remaps a value from one numeric range to another. For example, mapping `0.5`
//! from `[0, 1]` to `[0, 100]` yields `50.0`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that remaps a value from one range to another.
///
/// All inputs are converted to decimal. Returns an error if the input range
/// is zero (`in_min == in_max`). The formula is:
/// `out_min + (input - in_min) * (out_max - out_min) / (in_max - in_min)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathMapRange {}

impl OpNumberMathMapRange {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "map range".to_string(),
            description: "Remaps a value from one range to another.".to_string(),
        }
    }

    /// Creates the default input list: "input" (0.5), "in min" (0.0), "in max" (1.0),
    /// "out min" (0.0), and "out max" (100.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("in min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("in max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("out min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("out max".to_string(), Value::Decimal(100.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the map range operation.
    ///
    /// Remaps `input` from `[in_min, in_max]` to `[out_min, out_max]`.
    /// Returns an error if `in_min == in_max` (zero-width input range).
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let in_min_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let in_max_val = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let out_min_val = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let out_max_val = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_val.unwrap() else { unreachable!() };
        let Value::Decimal(in_min) = in_min_val.unwrap() else { unreachable!() };
        let Value::Decimal(in_max) = in_max_val.unwrap() else { unreachable!() };
        let Value::Decimal(out_min) = out_min_val.unwrap() else { unreachable!() };
        let Value::Decimal(out_max) = out_max_val.unwrap() else { unreachable!() };

        // validate input range is not zero
        if in_min == in_max {
            return Err(OperationError {
                input_errors: vec![], node_error: Some("Input range must not be zero.".to_string()),
            });
        }

        // run node
        let value = Value::Decimal(out_min + (input - in_min) * (out_max - out_min) / (in_max - in_min));

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
    async fn test_map_range_settings() {
        let s = OpNumberMathMapRange::settings();
        assert_eq!(s.name, "map range");
        assert_eq!(OpNumberMathMapRange::create_inputs().len(), 5);
        assert_eq!(OpNumberMathMapRange::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_map_range_midpoint() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.5), None, None),
            Input::new("in min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
            Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
        ];
        let result = OpNumberMathMapRange::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 50.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_map_range_at_min() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.0), None, None),
            Input::new("in min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
            Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
        ];
        let result = OpNumberMathMapRange::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_map_range_at_max() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(1.0), None, None),
            Input::new("in min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
            Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
        ];
        let result = OpNumberMathMapRange::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 100.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_map_range_zero_range_errors() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(0.5), None, None),
            Input::new("in min".to_string(), Value::Decimal(1.0), None, None),
            Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
            Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
        ];
        let result = OpNumberMathMapRange::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for zero input range");
    }
}
