//! Range membership comparison operation.
//!
//! Tests whether a value lies within an inclusive range: returns `true` when
//! `min <= value <= max`. All inputs are coerced to `Decimal` before
//! comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Range membership comparison node.
///
/// Outputs `true` when `min <= value <= max` after converting all inputs to decimals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareInRange {}

impl OpLogicCompareInRange {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "in range".to_string(),
            description: "Returns true if a value is within an inclusive min/max range.".to_string(),
            help: "Inclusive range test: true when min <= value <= max. All inputs are coerced to Decimal before comparison, so Bool and Integer sources cross-compare naturally.\n\nEquivalent to combining less-or-equal and greater-or-equal with an and gate, in a single node. Both endpoints are inclusive; if min is greater than max the range is empty and the output is always false. NaN inputs also produce false.".to_string(),
        }
    }

    /// Creates the default inputs: a `value` to test plus inclusive `min` and `max` bounds, all with drag-value UI.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("value".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to test for range membership."),
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Inclusive lower bound of the range."),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Inclusive upper bound of the range."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True when the value lies within [min, max].")
        ]
    }

    /// Converts all inputs to decimals and returns `true` if `min <= value <= max`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let value = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let min = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let max = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(value) = value.unwrap() else { unreachable!() };
        let Value::Decimal(min) = min.unwrap() else { unreachable!() };
        let Value::Decimal(max) = max.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(min <= value && value <= max) }],
        })
    }
}

#[cfg(test)]
#[path = "in_range_tests.rs"]
mod tests;
