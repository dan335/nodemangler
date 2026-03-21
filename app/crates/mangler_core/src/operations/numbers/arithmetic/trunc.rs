//! Truncation operation for the node graph.
//!
//! Removes the fractional part of a decimal, rounding toward zero. This is the
//! complement of the `frac` operation: `trunc(x) + frac(x) == x`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that truncates a decimal to its integer part.
///
/// Input is converted to decimal via `convert_input`. The result is computed
/// using `f32::trunc()`, which rounds toward zero. The output remains a decimal type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathTrunc {}

impl OpNumberMathTrunc {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "truncate".to_string(),
            description: "Truncates a decimal to its integer part.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input defaulting to 3.14.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the truncation: removes the fractional part of the input.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(val) = input_converted.unwrap() else { unreachable!() };

        let result = val.trunc();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "trunc_tests.rs"]
mod tests;
