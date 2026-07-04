//! Inequality comparison operation.
//!
//! Compares two values for inequality. Strings are compared directly; all other
//! types are coerced to `Decimal` before comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Inequality comparison node.
///
/// Outputs `true` when the two inputs hold different values. Inverse of the
/// equal operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareNotEqual {}

impl OpLogicCompareNotEqual {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "not equal".to_string(),
            description: "Returns true if two values are not equal.".to_string(),
            help: "Logical inverse of the equal node. Text-to-Text comparisons are performed directly as strings (case-sensitive), while all other combinations are coerced to Decimal first so Bool, Integer, and Decimal values can cross-compare (Bool::true -> 1.0, Bool::false -> 0.0).\n\nAs with equal, numeric differences that fall within floating-point rounding noise may produce a surprising true result. For a tolerance-based check, invert the approx equal node with not.".to_string(),
        }
    }

    /// Creates the default inputs: two decimal inputs `a` and `b` with drag-value UI, both defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("First value to compare for inequality."),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Second value to compare for inequality."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True when a and b are not equal.")
        ]
    }

    /// Compares the two inputs for inequality and returns a boolean result.
    ///
    /// String inputs are compared directly. All other types are converted to
    /// decimals first, allowing cross-type comparisons.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        // Text != Text: direct comparison without numeric coercion
        if let (Value::Text(a), Value::Text(b)) = (&inputs[0].value, &inputs[1].value) {
            return Ok(OperationResponse { 
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Bool(*a != *b) }],
            });
        }

        // For all numeric/bool types, convert both to Decimal and compare.
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
            responses: vec![OutputResponse { value: Value::Bool(a != b) }],
        })
    }
}

#[cfg(test)]
#[path = "not_equal_tests.rs"]
mod tests;
