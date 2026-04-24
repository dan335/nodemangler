//! Pi constant node for the node graph.
//!
//! Outputs the mathematical constant pi (3.14159...).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that outputs the constant pi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputPi {}

impl OpNumberInputPi {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "pi".to_string(),
            description: "Outputs the constant pi (3.14159...).".to_string(),
            help: "Emits the mathematical constant pi, the ratio of a circle's circumference to its diameter, at f32 precision (std::f32::consts::PI). Use it for angle conversions, trigonometric ranges, and radial patterns.".to_string(),
        }
    }

    /// Creates the default input list: no inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(std::f32::consts::PI), None)
                .with_description("The constant pi (approximately 3.14159).")
        ]
    }

    /// Executes the node: returns pi.
    pub async fn run(_inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(std::f32::consts::PI),
            }],
        })
    }
}

#[cfg(test)]
#[path = "pi_tests.rs"]
mod tests;
