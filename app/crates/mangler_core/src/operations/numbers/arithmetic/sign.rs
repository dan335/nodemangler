//! Sign operation for the node graph.
//!
//! Returns the sign of a number: -1, 0, or 1 for integers; -1.0 or 1.0 for
//! decimals (note: `f32::signum(0.0)` returns `1.0`, not `0.0`).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that returns the sign of a number.
///
/// Uses `i32::signum` for integers (returns -1, 0, or 1) and `f32::signum`
/// for decimals (returns -1.0 or 1.0; positive zero returns 1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathSign {}

impl OpNumberMathSign {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sign".to_string(),
            description: "Returns the sign of a number.".to_string(),
            help: "Returns -1 for negative inputs and 1 for positive inputs, preserving the input type (integer or decimal). For integers, zero returns 0; for decimals, f32::signum returns 1.0 for positive zero and -1.0 for negative zero.\n\nHandy as a multiplier to strip magnitude while keeping direction, or in combination with abs to reconstruct a signed value.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Number whose sign is evaluated.")
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Signum of the input: -1 if negative, 0/1 for zero (integers), 1 if positive.")
        ]
    }

    /// Executes the sign operation: returns the signum of the input.
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
            Value::Integer(a)=> Value::Integer((*a).signum()),
            Value::Decimal(a)=> Value::Decimal((*a).signum()),

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
#[path = "sign_tests.rs"]
mod tests;
