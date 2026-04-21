//! Minimum operation for the node graph.
//!
//! Returns the smaller of two numbers. Mixed integer/decimal types promote to decimal.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that returns the minimum of two numbers.
///
/// Supports integer and decimal types. Mixed types promote to decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathMin {}

impl OpNumberMathMin {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "min".to_string(),
            description: "Returns the minimum of two numbers.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (a and b).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("b".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the min operation: returns the smaller of `a` and `b`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal((*a as f32).min(*b)),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a.min(b)),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(a.min(*b)),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(a.min(*b as f32)),

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
#[path = "min_tests.rs"]
mod tests;
