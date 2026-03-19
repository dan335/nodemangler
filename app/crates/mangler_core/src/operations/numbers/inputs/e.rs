//! Euler's number constant node for the node graph.
//!
//! Outputs Euler's number e (2.71828...).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that outputs Euler's number e.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputE {}

impl OpNumberInputE {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "e".to_string(),
            description: "Outputs Euler's number e (2.71828...).".to_string(),
        }
    }

    /// Creates the default input list: no inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(std::f32::consts::E), None)
        ]
    }

    /// Executes the node: returns e.
    pub async fn run(_inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(std::f32::consts::E),
            }],
        })
    }
}

#[cfg(test)]
#[path = "e_tests.rs"]
mod tests;
