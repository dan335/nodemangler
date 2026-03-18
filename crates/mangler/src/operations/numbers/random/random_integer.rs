//! Random integer generation operation for the node graph.
//!
//! Generates a random integer in the range `[min, max)` each time the node is
//! triggered. If `max <= min`, `max` is clamped to `min + 1` so the range is
//! always valid.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that generates a random integer in `[min, max)`.
///
/// Takes a trigger input plus `min` and `max` integer bounds. Uses
/// `fastrand::i32(min..max)`. When `max <= min`, `max` is clamped to
/// `min.saturating_add(1)` to ensure a valid range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberRandomInteger {}

impl OpNumberRandomInteger {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random integer".to_string(),
            description: "Generates a random integer number between min and max.".to_string(),
        }
    }

    /// Creates the default input list: trigger, `min` (i32::MIN), and `max` (i32::MAX).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(i32::MIN), None, None),
            Input::new("max".to_string(), Value::Integer(i32::MAX), None, None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the node: generates a random integer in `[min, max)`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let min_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let max_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(minimum) = min_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut maximum) = max_converted.unwrap() else { unreachable!() };

        // run node
        maximum = maximum.max(minimum.saturating_add(1));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(fastrand::i32(minimum..maximum)),
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
    async fn test_random_integer_in_range() {
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(0), None, None),
            Input::new("max".to_string(), Value::Integer(100), None, None),
        ];
        let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert!(*v >= 0 && *v < 100),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_random_integer_min_equals_max() {
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(5), None, None),
            Input::new("max".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_random_integer_settings() {
        let s = OpNumberRandomInteger::settings();
        assert_eq!(s.name, "random integer");
        assert_eq!(OpNumberRandomInteger::create_inputs().len(), 3);
        assert_eq!(OpNumberRandomInteger::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_random_integer_negative_range() {
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(-100), None, None),
            Input::new("max".to_string(), Value::Integer(-10), None, None),
        ];
        let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert!(*v >= -100 && *v < -10, "Got {}", v),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_random_integer_min_greater_than_max_clamped() {
        // Implementation clamps max to min+1 when max <= min
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(10), None, None),
            Input::new("max".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
        // When max < min, max gets clamped to min.saturating_add(1), so result must be min
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 10),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_random_integer_unit_range() {
        // min=0, max=1: result should always be 0
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(0), None, None),
            Input::new("max".to_string(), Value::Integer(1), None, None),
        ];
        for _ in 0..10 {
            let mut i = inputs.clone();
            let result = OpNumberRandomInteger::run(&mut i).await.unwrap();
            match &result.responses[0].value {
                Value::Integer(v) => assert_eq!(*v, 0, "Range [0,1) should always give 0"),
                other => panic!("Expected Integer, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_random_integer_multiple_calls_in_range() {
        for _ in 0..20 {
            let mut inputs = vec![
                Input::new("generate".to_string(), Value::Trigger, None, None),
                Input::new("min".to_string(), Value::Integer(0), None, None),
                Input::new("max".to_string(), Value::Integer(100), None, None),
            ];
            let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
            match &result.responses[0].value {
                Value::Integer(v) => assert!(*v >= 0 && *v < 100, "Out-of-range: {}", v),
                other => panic!("Expected Integer, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_random_integer_from_decimal_range() {
        // Decimal inputs for min/max are converted to Integer
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert!(*v >= 0 && *v < 10, "Got {}", v),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
