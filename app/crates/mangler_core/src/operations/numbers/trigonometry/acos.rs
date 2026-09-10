//! Arccosine operation for the node graph.
//!
//! Computes the arccosine (inverse cosine) of a value. Input must be in [-1, 1].

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the arccosine (inverse cosine) of a value.
///
/// Input must be in the range [-1, 1]. Returns an error for out-of-range inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAcos {}

impl OpNumberTrigAcos {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "acos".to_string(),
            description: "Computes the arccosine (inverse cosine). Input must be in [-1, 1].".to_string(),
            help: "Returns the angle in radians whose cosine equals the input, always in the range [0, pi].\n\nInput is not clamped: values outside [-1, 1] return a node error rather than silently producing NaN. Pair with cos as its inverse for the principal branch.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Cosine value to invert; must lie in [-1, 1]."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Arccosine of the input in radians, in the range [0, pi].")
        ]
    }

    /// Executes the arccosine operation. Returns an error if input is outside [-1, 1].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs; Decimal(input) = 0 }

        // Validate input range for arccosine.
        if !(-1.0..=1.0).contains(&input) {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some(format!("acos input must be in [-1, 1], got {}", input)),
            });
        }

        let result = input.acos();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "acos_tests.rs"]
mod tests;
