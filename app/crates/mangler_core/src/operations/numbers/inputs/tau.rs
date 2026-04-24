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
            help: "Emits the mathematical constant tau, equal to 2 * pi or one full turn in radians, at f32 precision (std::f32::consts::TAU).\n\nOften more natural than pi for angular work: a full rotation is one tau, a quarter turn is tau/4, and so on. Handy for seamless loops and circular parameterizations.".to_string(),
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
                .with_description("The constant tau, equal to 2 * pi (approximately 6.28318).")
        ]
    }

    /// Executes the node: returns tau.
    pub async fn run(_inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse { 
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
