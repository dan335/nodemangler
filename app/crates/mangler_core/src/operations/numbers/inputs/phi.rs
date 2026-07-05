//! Phi (golden ratio) constant node for the node graph.
//!
//! Outputs the golden ratio phi (1.6180339887...).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that outputs the golden ratio phi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputPhi {}

impl OpNumberInputPhi {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "phi".to_string(),
            description: "Outputs the golden ratio phi (1.61803...).".to_string(),
            help: "Emits the golden ratio phi = (1 + sqrt(5)) / 2, approximately 1.6180339887, at f32 precision. It is the positive solution of x^2 = x + 1 and the limit of the ratio of consecutive Fibonacci numbers.\n\nUse it for aesthetically balanced proportions, spiral and phyllotaxis patterns, and quasi-random sampling via the golden-angle increment.".to_string(),
        }
    }

    /// Creates the default input list: no inputs.
    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal((1.0 + 5.0f32.sqrt()) / 2.0), None)
                .with_description("The golden ratio phi (approximately 1.61803).")
        ]
    }

    /// Executes the node: returns phi.
    pub async fn run(_inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal((1.0 + 5.0f32.sqrt()) / 2.0),
            }],
        })
    }
}

#[cfg(test)]
#[path = "phi_tests.rs"]
mod tests;
