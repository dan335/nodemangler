//! Clamp operation for the node graph.
//!
//! Restricts a value to lie within a specified `[min, max]` range.
//! The min and max bounds are converted to decimals for comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that clamps a number between a minimum and maximum.
///
/// Accepts integer or decimal input. The `min` and `max` bounds are converted
/// to decimal for the comparison. Integer inputs produce integer outputs
/// (the clamped value is rounded back to i32).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathClamp {}

impl OpNumberMathClamp {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clamp".to_string(),
            description: "Clamps a number between two values.".to_string(),
        }
    }

    /// Creates the default input list: value `a`, `min` (0.0), and `max` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the clamp: restricts input `a` to the `[min, max]` range.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(min)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert 'min' to Decimal.".to_string())], node_error: None })};
        let Ok(Value::Decimal(max)) = inputs[2].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(2, "Unable to convert 'max' to Decimal.".to_string())], node_error: None })};

        let value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer((*a as f32).clamp(min, max).round() as i32),
            Value::Decimal(a) => Value::Decimal(a.clone().clamp(min, max)),

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
    async fn test_clamp_settings() {
        let s = OpNumberMathClamp::settings();
        assert_eq!(s.name, "clamp");
        assert_eq!(OpNumberMathClamp::create_inputs().len(), 3);
        assert_eq!(OpNumberMathClamp::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_clamp_within_range() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(5.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_below_min() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(-5.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_above_max() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(15.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_exactly_at_min() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(0.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_exactly_at_max() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(10.0), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_integer_below_min() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-10), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(100.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_integer_above_max() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(200), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(100.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 100),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_negative_range() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(-5.0), None, None),
            Input::new("min".to_string(), Value::Decimal(-10.0), None, None),
            Input::new("max".to_string(), Value::Decimal(-1.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-5.0)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_invalid_type_returns_error() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Bool(true), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }

    #[tokio::test]
    async fn test_clamp_min_from_integer() {
        // min/max accept integer via try_convert_to
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(5.0), None, None),
            Input::new("min".to_string(), Value::Integer(2), None, None),
            Input::new("max".to_string(), Value::Integer(10), None, None),
        ];
        let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
