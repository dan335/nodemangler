//! Hyperbolic cosine operation for the node graph.
//!
//! Computes the hyperbolic cosine of a value.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the hyperbolic cosine of a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigCosh {}

impl OpNumberTrigCosh {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cosh".to_string(),
            description: "Computes the hyperbolic cosine of a value.".to_string(),
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

    /// Executes the hyperbolic cosine operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.cosh();

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "cosh_tests.rs"]
mod tests;
