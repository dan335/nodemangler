//! Tau constant node for the node graph.
//!
//! Outputs the mathematical constant tau (2*pi = 6.28318...).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that outputs the constant tau.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputTau {}

impl OpNumberInputTau {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tau".to_string(),
            description: "Outputs the constant tau (2*pi = 6.28318...).".to_string(),
        }
    }

    /// Creates the default input list: no inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(std::f32::consts::TAU), None)
        ]
    }

    /// Executes the node: returns tau.
    pub async fn run(_inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(std::f32::consts::TAU),
            }],
        })
    }
}

#[cfg(test)]
#[path = "tau_tests.rs"]
mod tests;
