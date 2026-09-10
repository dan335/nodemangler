//! Integer input node operation.
//!
//! Provides a single integer value to the graph. Accepts integer or decimal inputs
//! (decimals are truncated to integers via type conversion).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "integer_tests.rs"]
mod tests;

/// Node operation that outputs an integer value.
///
/// Passes through a single integer input as the output. Input values of other
/// numeric types are converted to integers (e.g., decimals are truncated).
#[derive(Clone, Serialize, Deserialize)]
pub struct OpNumberInputInteger {}


impl OpNumberInputInteger {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "integer".to_string(),
            description: "An integer number input.".to_string(),
            help: "Emits a single signed 32-bit integer onto the graph. Use this node to supply counts, indices, seeds, or other whole-number parameters.\n\nConnections from decimal inputs are truncated toward zero (not rounded); booleans become 0 or 1. Values outside the i32 range cannot be represented and will be clamped or error during conversion.".to_string(),
        }
    }

    /// Creates the default input list: a single integer drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Integer value to emit; decimals are truncated to integers.")
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(i32::default()), None)
                .with_description("The integer value from the input, passed through.")
        ]
    }

    /// Executes the node: converts the input to an integer and passes it through.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(input) = 0,
        }

        // run node
        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(input),
            }],
        })
    }
}
