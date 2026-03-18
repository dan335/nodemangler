//! Equality comparison operation.
//!
//! Compares two values for equality. Strings are compared directly; all other
//! types (integers, decimals, booleans) are coerced to `Decimal` before
//! comparison, with booleans mapping to 1.0 (true) and 0.0 (false).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Equality comparison node.
///
/// Outputs `true` when both inputs hold equal values. Supports string-to-string
/// and numeric-to-numeric (via decimal coercion) comparisons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareEqual {}

impl OpLogicCompareEqual {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "equal".to_string(),
            description: "Returns true if two values are equal.".to_string(),
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

    /// Compares the two inputs for equality and returns a boolean result.
    ///
    /// String inputs are compared directly. All other types are converted to
    /// decimals first, allowing cross-type comparisons (e.g., `Bool(true) == Integer(1)`).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        // Text == Text: direct comparison without numeric coercion
        if let (Value::Text(a), Value::Text(b)) = (&inputs[0].value, &inputs[1].value) {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Bool(*a == *b) }],
            });
        }

        // For all numeric/bool types, convert both to Decimal and compare.
        // This handles: Int==Int, Dec==Dec, Int==Dec, Bool==Int, Bool==Dec, Bool==Bool
        // Bool converts as: true → 1.0, false → 0.0 (matching Rust/JS semantics)
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
            responses: vec![OutputResponse { value: Value::Bool(a == b) }],
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
    async fn test_equal_integers_true() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_integers_false() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(10));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_decimals_true() {
        let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(3.14));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_decimals_false() {
        let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(2.71));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_mixed_int_decimal() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Decimal(5.0));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_bools() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_text() {
        let mut inputs = make_inputs(Value::Text("hello".to_string()), Value::Text("hello".to_string()));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_text_false() {
        let mut inputs = make_inputs(Value::Text("hello".to_string()), Value::Text("world".to_string()));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    // Bool/Numeric mixed: true converts to 1.0, false to 0.0 (JS/Rust semantics)
    #[tokio::test]
    async fn test_equal_bool_true_decimal_one() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(1.0));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_bool_true_decimal_point_one() {
        // 0.1 != true  (true == 1.0, not 0.1)
        let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_bool_true_decimal_neg_point_one() {
        // -0.1 != true
        let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(-0.1));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_decimal_point_one_bool_true() {
        // symmetric: 0.1 != true
        let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_decimal_neg_point_one_bool_true() {
        // symmetric: -0.1 != true
        let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Bool(true));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_bool_false_decimal_zero() {
        // false == 0.0
        let mut inputs = make_inputs(Value::Bool(false), Value::Decimal(0.0));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_bool_true_integer_one() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Integer(1));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_bool_true_integer_zero() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Integer(0));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_bool_false_integer_zero() {
        let mut inputs = make_inputs(Value::Bool(false), Value::Integer(0));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_settings() {
        let s = OpLogicCompareEqual::settings();
        assert_eq!(s.name, "equal");
        assert_eq!(OpLogicCompareEqual::create_inputs().len(), 2);
        assert_eq!(OpLogicCompareEqual::create_outputs().len(), 1);
    }
}
