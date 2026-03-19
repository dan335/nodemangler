//! Decimal input node operation.
//!
//! Provides a single decimal (f32) value to the graph. Accepts integer or decimal
//! inputs (integers are promoted to decimals via type conversion).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "decimal_tests.rs"]
mod tests;

/// Node operation that outputs a decimal (f32) value.
///
/// Passes through a single decimal input as the output. Input values of other
/// numeric types are converted to decimals (e.g., integers are widened to f32).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputDecimal {}

impl OpNumberInputDecimal {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "decimal".to_string(),
            description: "A decimal number input.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the node: converts the input to a decimal and passes it through.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        // run node
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(input),
            }],
        })
    }
}
