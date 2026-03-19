//! Natural logarithm (`ln`) operation for the node graph.
//!
//! Computes `ln(x)` (the logarithm base e). Returns an error if the input is
//! less than or equal to zero.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the natural logarithm of a number.
///
/// Input must be strictly positive. Uses f64 precision internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLn {}

impl OpNumberMathLn {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ln".to_string(),
            description: "Computes the natural logarithm.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input defaulting to e (2.718).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(std::f32::consts::E), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the natural log: computes `ln(input)`, erroring if input <= 0.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        if input <= 0.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Input must be greater than 0.".to_string()) });
        }

        let result = (input as f64).ln() as f32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "ln_tests.rs"]
mod tests;
