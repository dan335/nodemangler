//! Less-than-or-equal comparison operation.
//!
//! Compares two numeric values and returns `true` when `a <= b`. Both inputs
//! are coerced to `Decimal` before comparison.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Less-than-or-equal comparison node.
///
/// Outputs `true` when `a <= b` after converting both inputs to decimals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareLessEqual {}

impl OpLogicCompareLessEqual {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "less or equal".to_string(),
            description: "Returns true if a is less than or equal to b.".to_string(),
            help: "Both inputs are coerced to Decimal before evaluating a <= b. Equal values produce true, making this the inclusive counterpart to less-than.\n\nBool, Integer, and Decimal sources are all supported through the normal numeric coercion rules (Bool::true -> 1.0, Bool::false -> 0.0).".to_string(),
        }
    }

    /// Creates the default inputs: two decimal inputs `a` and `b` with drag-value UI, both defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Left-hand value in the comparison a <= b."),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Right-hand value in the comparison a <= b."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True when a is less than or equal to b.")
        ]
    }

    /// Converts both inputs to decimals and returns `true` if `a <= b`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Decimal(a) = 0,
            Decimal(b) = 1,
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(a <= b) }],
        })
    }
}

#[cfg(test)]
#[path = "less_equal_tests.rs"]
mod tests;
