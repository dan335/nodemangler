//! Average operation for the node graph.
//!
//! Computes the arithmetic mean of two numbers. Both inputs are converted to
//! decimal before the computation.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the average (mean) of two numbers.
///
/// Both inputs are converted to decimal. The result is `(a + b) / 2.0`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathAverage {}

impl OpNumberMathAverage {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "average".to_string(),
            description: "Computes the average (mean) of two numbers.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the average operation: computes `(a + b) / 2.0`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let a_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let b_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(a) = a_val.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_val.unwrap() else { unreachable!() };

        // run node
        let value = Value::Decimal((a + b) / 2.0);

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "average_tests.rs"]
mod tests;
