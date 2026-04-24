//! Increment operation for the node graph.
//!
//! Adds 1 to an integer or decimal value. For strings, appends " +1".

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that increments a value by 1.
///
/// For integers, adds 1. For decimals, adds 1.0. For strings, appends " +1".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathIncrement {}

impl OpNumberMathIncrement {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "increment".to_string(),
            description: "Increments a number by 1.".to_string(),
            help: "Adds 1 to a numeric input while preserving its type: integers gain 1 exactly, decimals gain 1.0.\n\nText inputs are handled specially by appending \" +1\" rather than attempting arithmetic, which can be handy for generating labels.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Value to increment; integers and decimals gain 1, text appends \" +1\"."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Input value with 1 added.")
        ]
    }

    /// Executes the increment: adds 1 to the input value.
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
            Value::Integer(a) => Value::Integer(*a + 1),

            Value::Decimal(a) => Value::Decimal(*a + 1.0),

            Value::Text(a) => Value::Text(format!("{} {}", *a, "+1")),

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
#[path = "increment_tests.rs"]
mod tests;
