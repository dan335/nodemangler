//! Floor operation for the node graph.
//!
//! Returns the largest integer less than or equal to the input.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that returns the floor of a number.
///
/// Rounds down to the nearest integer value, returned as a decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathFloor {}

impl OpNumberMathFloor {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "floor".to_string(),
            description: "Rounds down to the nearest integer.".to_string(),
            help: "Returns the largest integer less than or equal to the input, using f32::floor. The result is returned as a decimal so it can feed directly into other floating-point math.\n\nFlooring always rounds toward negative infinity, so floor(-1.5) yields -2.0, not -1.0. Pair with ceil or round when you need different rounding semantics.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Number to round down to the nearest integer."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Largest integer value less than or equal to the input.")
        ]
    }

    /// Executes the floor operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(input.floor()),
            }],
        })
    }
}

#[cfg(test)]
#[path = "floor_tests.rs"]
mod tests;
