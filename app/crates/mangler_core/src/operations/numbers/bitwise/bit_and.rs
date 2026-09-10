//! Bitwise AND operation for the node graph.
//!
//! Computes the bitwise AND of two integers.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the bitwise AND of two integers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberBitwiseAnd {}

impl OpNumberBitwiseAnd {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bitwise and".to_string(),
            description: "Computes the bitwise AND of two integers.".to_string(),
            help: "Returns a & b, setting each output bit only when both corresponding input bits are 1.\n\nCommonly used for masking: AND-ing with 0xFF keeps the low byte, AND-ing with (1 << n) - 1 keeps the low n bits. Operates on signed 32-bit integers, so the sign bit is treated like any other bit.".to_string(),
        }
    }

    /// Creates the default input list: two integer drag-value inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("First integer operand for the bitwise AND."),
            Input::new("b".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Second integer operand for the bitwise AND."),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Bitwise AND of a and b (a & b).")
        ]
    }

    /// Executes the bitwise AND operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Integer(a) = 0,
            Integer(b) = 1,
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(a & b),
            }],
        })
    }
}

#[cfg(test)]
#[path = "bit_and_tests.rs"]
mod tests;
