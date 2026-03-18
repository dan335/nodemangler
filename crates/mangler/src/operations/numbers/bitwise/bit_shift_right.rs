//! Bitwise right shift operation for the node graph.
//!
//! Shifts an integer right by a specified number of bits.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that shifts an integer right by a specified number of bits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberBitwiseShiftRight {}

impl OpNumberBitwiseShiftRight {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "shift right".to_string(),
            description: "Shifts an integer right by a specified number of bits.".to_string(),
        }
    }

    /// Creates the default input list: an integer input and a shift amount.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("amount".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the bitwise right shift operation.
    ///
    /// The shift amount is validated to be in the 0..=31 range. If outside
    /// that range, a node error is returned.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(input) = input_converted.unwrap() else { unreachable!() };
        let Value::Integer(amount) = amount_converted.unwrap() else { unreachable!() };

        // Validate shift amount is within safe range.
        if amount < 0 || amount > 31 {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some(format!("Shift amount must be between 0 and 31, got {}", amount)),
            });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(input >> amount),
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
    async fn test_shift_right_settings() {
        let s = OpNumberBitwiseShiftRight::settings();
        assert_eq!(s.name, "shift right");
        assert_eq!(OpNumberBitwiseShiftRight::create_inputs().len(), 2);
        assert_eq!(OpNumberBitwiseShiftRight::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_shift_right_by_four() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Integer(16), None, None),
            Input::new("amount".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpNumberBitwiseShiftRight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_shift_right_by_one() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Integer(8), None, None),
            Input::new("amount".to_string(), Value::Integer(1), None, None),
        ];
        let result = OpNumberBitwiseShiftRight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 4),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_shift_right_zero() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Integer(0), None, None),
            Input::new("amount".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberBitwiseShiftRight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_shift_right_negative_amount() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Integer(16), None, None),
            Input::new("amount".to_string(), Value::Integer(-1), None, None),
        ];
        let result = OpNumberBitwiseShiftRight::run(&mut inputs).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.node_error.is_some());
    }

    #[tokio::test]
    async fn test_shift_right_overflow_amount() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Integer(16), None, None),
            Input::new("amount".to_string(), Value::Integer(32), None, None),
        ];
        let result = OpNumberBitwiseShiftRight::run(&mut inputs).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.node_error.is_some());
    }
}
