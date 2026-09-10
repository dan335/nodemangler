//! Bitwise NOT operation for the node graph.
//!
//! Computes the bitwise complement of an integer.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the bitwise NOT (complement) of an integer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberBitwiseNot {}

impl OpNumberBitwiseNot {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bitwise not".to_string(),
            description: "Computes the bitwise NOT (complement) of an integer.".to_string(),
            help: "Returns !a, flipping every bit of the 32-bit signed integer.\n\nBecause of two's complement, !a is equivalent to -a - 1: for example, !0 is -1 and !5 is -6. For a logical boolean complement, use the logic category's `not` node instead.".to_string(),
        }
    }

    /// Creates the default input list: a single integer drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Integer whose bits will be inverted."),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Bitwise complement of a (!a), flipping every bit.")
        ]
    }

    /// Executes the bitwise NOT operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(a) = 0,
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(!a),
            }],
        })
    }
}

#[cfg(test)]
#[path = "bit_not_tests.rs"]
mod tests;
