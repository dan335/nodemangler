//! Negate operation for the node graph.
//!
//! Flips the sign of a number: positive becomes negative, negative becomes
//! positive. Works with both integer and decimal types.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that negates a number (flips its sign).
///
/// Supports integer and decimal types. Integer inputs produce integer outputs,
/// decimal inputs produce decimal outputs. Returns an error for unsupported types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathNegate {}

impl OpNumberMathNegate {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "negate".to_string(),
            description: "Negates a number (flips sign).".to_string(),
            help: "Flips the sign of the input: positive values become negative and vice versa, while zero is unchanged. The output type matches the input: integer stays integer, decimal stays decimal.\n\nEquivalent to multiplying by -1 but slightly clearer in graphs. Note that i32::MIN cannot be negated in integer arithmetic and will wrap, following native Rust semantics.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Number whose sign will be flipped.")
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Negation of the input; type matches the input type.")
        ]
    }

    /// Executes the negate operation: returns the negation of the input value.
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
            Value::Integer(a) => Value::Integer(-a),
            Value::Decimal(a) => Value::Decimal(-a),

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
#[path = "negate_tests.rs"]
mod tests;
