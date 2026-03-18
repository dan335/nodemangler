//! Less-than comparison operation.
//!
//! Compares two numeric values and returns `true` when `a < b`. Both inputs
//! are coerced to `Decimal` before comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Less-than comparison node.
///
/// Outputs `true` when `a < b` after converting both inputs to decimals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareLessThan {}

impl OpLogicCompareLessThan {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "less than".to_string(),
            description: "Returns true if a is less than b.".to_string(),
        }
    }

    /// Creates the default inputs: two decimal inputs `a` and `b` with drag-value UI, both defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
        ]
    }

    /// Converts both inputs to decimals and returns `true` if `a < b`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let b = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(a) = a.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(a < b) }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn make_inputs(a: Value, b: Value) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), a, None, None),
            Input::new("b".to_string(), b, None, None),
        ]
    }

    #[tokio::test]
    async fn test_less_than_true() {
        let mut inputs = make_inputs(Value::Integer(3), Value::Integer(5));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_less_than_false() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(3));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_less_than_equal_false() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_less_than_decimals() {
        let mut inputs = make_inputs(Value::Decimal(1.5), Value::Decimal(2.5));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_less_than_mixed() {
        let mut inputs = make_inputs(Value::Integer(2), Value::Decimal(2.5));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_less_than_negative() {
        let mut inputs = make_inputs(Value::Integer(-10), Value::Integer(-5));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    // Bool/Numeric mixed: true converts to 1.0, false to 0.0
    #[tokio::test]
    async fn test_less_than_decimal_point_one_lt_bool_true() {
        // 0.1 < true (0.1 < 1.0) → true
        let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_less_than_decimal_neg_point_one_lt_bool_true() {
        // -0.1 < true (-0.1 < 1.0) → true
        let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Bool(true));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_less_than_bool_true_lt_decimal_point_one() {
        // true < 0.1 (1.0 < 0.1) → false
        let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_less_than_bool_false_lt_bool_true() {
        // false < true (0 < 1) → true
        let mut inputs = make_inputs(Value::Bool(false), Value::Bool(true));
        let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_less_than_settings() {
        let s = OpLogicCompareLessThan::settings();
        assert_eq!(s.name, "less than");
    }
}
