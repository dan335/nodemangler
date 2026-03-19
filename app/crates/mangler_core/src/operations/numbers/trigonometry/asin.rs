//! Arcsine operation for the node graph.
//!
//! Computes the arcsine (inverse sine) of a value. Input must be in [-1, 1].

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the arcsine (inverse sine) of a value.
///
/// Input must be in the range [-1, 1]. Returns an error for out-of-range inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAsin {}

impl OpNumberTrigAsin {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "asin".to_string(),
            description: "Computes the arcsine (inverse sine). Input must be in [-1, 1].".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the arcsine operation. Returns an error if input is outside [-1, 1].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        // Validate input range for arcsine.
        if !(-1.0..=1.0).contains(&input) {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some(format!("asin input must be in [-1, 1], got {}", input)),
            });
        }

        let result = input.asin();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "asin_tests.rs"]
mod tests;
