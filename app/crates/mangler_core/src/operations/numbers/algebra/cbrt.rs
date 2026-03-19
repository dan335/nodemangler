//! Cube root operation for the node graph.
//!
//! Computes the cube root of a number. Unlike square root, cube root handles
//! negative inputs (returning a negative result).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the cube root of a number.
///
/// Always returns a decimal. Integer inputs are cast to f32 before computing.
/// Negative inputs produce negative results (e.g., cbrt(-8) = -2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathCbrt {}

impl OpNumberMathCbrt {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cube root".to_string(),
            description: "Returns the cube root of a number.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the cube root: computes `cbrt(a)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match &inputs[0].value {

            Value::Integer(a) => Value::Decimal((*a as f32).cbrt()),
            Value::Decimal(a) => Value::Decimal(a.cbrt()),

            _ => {return Err(OperationError {
                input_errors: vec![], node_error: Some("Error converting.".to_string()),
            });}
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "cbrt_tests.rs"]
mod tests;
