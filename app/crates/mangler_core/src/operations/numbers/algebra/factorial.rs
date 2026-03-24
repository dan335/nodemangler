//! Factorial operation for the node graph.
//!
//! Computes `n!` for non-negative integers. The input is clamped to `[0, 12]`
//! because `12! = 479,001,600` is the largest factorial that fits in an `i32`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the factorial of an integer.
///
/// Input is clamped to `[0, 12]` to prevent i32 overflow. Decimal inputs are
/// first converted (truncated) to integer via `convert_input`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathFactorial {}

impl OpNumberMathFactorial {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "factorial".to_string(),
            description: "Computes the factorial of an integer.".to_string(),
        }
    }

    /// Creates the default input list: a single integer input clamped to `[0, 12]`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(5), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 12.0)) }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the factorial: computes `n!` with input clamped to `[0, 12]`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(val) = input_converted.unwrap() else { unreachable!() };

        let val = val.clamp(0, 12); // 12! = 479001600, max that fits in i32

        let mut result: i32 = 1;
        for i in 2..=(val) {
            result *= i;
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "factorial_tests.rs"]
mod tests;
