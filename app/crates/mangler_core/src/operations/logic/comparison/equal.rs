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
            help: "When both inputs are Text, they are compared as strings (case-sensitive, exact match). Otherwise both inputs are coerced to Decimal and compared numerically, which lets Bool, Integer, and Decimal values cross-compare naturally (Bool::true maps to 1.0, Bool::false to 0.0).\n\nBecause numeric comparison uses exact f32 equality, floating-point rounding can cause values that look identical to differ. For noisy floats, use the approx equal node instead.".to_string(),
        }
    }

    /// Creates the default inputs: two decimal inputs `a` and `b` with drag-value UI, both defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("First value to compare for equality."),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Second value to compare for equality."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True when a equals b.")
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
#[path = "equal_tests.rs"]
mod tests;
