//! Bitwise left shift operation for the node graph.
//!
//! Shifts an integer left by a specified number of bits.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that shifts an integer left by a specified number of bits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberBitwiseShiftLeft {}

impl OpNumberBitwiseShiftLeft {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "shift left".to_string(),
            description: "Shifts an integer left by a specified number of bits.".to_string(),
            help: "Returns input << amount, moving bits toward the high end and filling low bits with zeros. Each position shifted is equivalent to multiplying by two.\n\namount must be in the range 0..=31; values outside that range produce a node error rather than triggering Rust's undefined-shift behavior. Bits shifted into or past the sign position can flip the integer negative (no overflow check is performed).".to_string(),
        }
    }

    /// Creates the default input list: an integer input and a shift amount.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Integer whose bits will be shifted left."),
            Input::new("amount".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Number of bit positions to shift left; must be between 0 and 31."),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Input shifted left by amount bits (input << amount).")
        ]
    }

    /// Executes the bitwise left shift operation.
    ///
    /// The shift amount is validated to be in the 0..=31 range. If outside
    /// that range, a node error is returned.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(input) = input_converted.unwrap() else { unreachable!() };
        let Value::Integer(amount) = amount_converted.unwrap() else { unreachable!() };

        // Validate shift amount is within safe range.
        if !(0..=31).contains(&amount) {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some(format!("Shift amount must be between 0 and 31, got {}", amount)),
            });
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(input << amount),
            }],
        })
    }
}

#[cfg(test)]
#[path = "bit_shift_left_tests.rs"]
mod tests;
