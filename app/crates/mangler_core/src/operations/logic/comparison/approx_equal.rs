//! Approximate equality comparison operation.
//!
//! Compares two numeric values with a tolerance: returns `true` when
//! `|a - b| <= tolerance`. All inputs are coerced to `Decimal` before
//! comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Approximate equality comparison node.
///
/// Outputs `true` when `|a - b| <= tolerance` after converting all inputs to decimals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareApproxEqual {}

impl OpLogicCompareApproxEqual {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "approx equal".to_string(),
            description: "Returns true if two values are equal within a tolerance.".to_string(),
            help: "Tolerance-based equality: true when |a - b| <= tolerance. All inputs are coerced to Decimal before comparison.\n\nUse this instead of the equal node when comparing computed decimals, where floating-point rounding makes exact f32 equality unreliable. A tolerance of 0 behaves like exact equality; a negative tolerance always returns false. NaN inputs always compare as not equal.".to_string(),
        }
    }

    /// Creates the default inputs: decimal values `a` and `b` plus a `tolerance`, all with drag-value UI.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("First value to compare."),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Second value to compare."),
            Input::new("tolerance".to_string(), Value::Decimal(0.000001), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Maximum absolute difference for a and b to count as equal."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True when |a - b| is within the tolerance.")
        ]
    }

    /// Converts all inputs to decimals and returns `true` if `|a - b| <= tolerance`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Decimal(a) = 0,
            Decimal(b) = 1,
            Decimal(tolerance) = 2,
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool((a - b).abs() <= tolerance) }],
        })
    }
}

#[cfg(test)]
#[path = "approx_equal_tests.rs"]
mod tests;
