//! Cast-to-decimal operation for the node graph.
//!
//! Converts a numeric value to a decimal (f32) using `try_convert_to`.
//! Integer inputs are widened; decimal inputs pass through unchanged.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to decimal (f32).
///
/// Uses `Value::try_convert_to(ValueType::Decimal)` for the conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCastToDecimal {}

impl OpNumberCastToDecimal {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to decimal".to_string(),
            description: "Converts a number to a decimal.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(f32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the cast: converts the input to a decimal via `try_convert_to`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(n)) = inputs[0].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to decimal.".to_string())], node_error: None })};

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(n),
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
    async fn test_to_decimal_settings() {
        let s = OpNumberCastToDecimal::settings();
        assert_eq!(s.name, "to decimal");
        assert_eq!(OpNumberCastToDecimal::create_inputs().len(), 1);
        assert_eq!(OpNumberCastToDecimal::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_to_decimal_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 42.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_passthrough() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.14).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_from_negative_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-7), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-7.0)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_zero_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_zero_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_large_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(i32::MAX / 2), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v > 0.0),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_decimal_negative_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-99.5), None, None)];
        let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-99.5)).abs() < 1e-3),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
