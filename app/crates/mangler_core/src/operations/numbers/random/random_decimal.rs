//! Random decimal generation operation for the node graph.
//!
//! Generates a random decimal in `[0.0, 1.0)` each time the node is triggered.
//! Uses `fastrand::f32()` for fast, non-cryptographic randomness.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that generates a random decimal in `[0.0, 1.0)`.
///
/// Takes a single trigger input and outputs a random `f32` via `fastrand::f32()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberRandomDecimal {}

impl OpNumberRandomDecimal {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random decimal".to_string(),
            description: "Generates a random decimal number between 0 and 1.".to_string(),
        }
    }

    /// Creates the default input list: a single trigger input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the node: generates and outputs a random decimal in `[0.0, 1.0)`.
    pub async fn run(_inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(fastrand::f32()),
            }],
        })
    }
}

#[cfg(test)]
#[path = "random_decimal_tests.rs"]
mod tests;
