//! Random integer generation operation for the node graph.
//!
//! Generates a random integer in the range `[min, max)` each time the node is
//! triggered. If `max <= min`, `max` is clamped to `min + 1` so the range is
//! always valid.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that generates a random integer in `[min, max)`.
///
/// Takes a trigger input plus `min` and `max` integer bounds. Uses
/// `fastrand::i32(min..max)`. When `max <= min`, `max` is clamped to
/// `min.saturating_add(1)` to ensure a valid range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberRandomInteger {}

impl OpNumberRandomInteger {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random integer".to_string(),
            description: "Generates a random integer number between min and max.".to_string(),
        }
    }

    /// Creates the default input list: trigger, `min` (i32::MIN), and `max` (i32::MAX).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(i32::MIN), None, None),
            Input::new("max".to_string(), Value::Integer(i32::MAX), None, None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the node: generates a random integer in `[min, max)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let min_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let max_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(minimum) = min_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut maximum) = max_converted.unwrap() else { unreachable!() };

        // run node
        maximum = maximum.max(minimum.saturating_add(1));

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(fastrand::i32(minimum..maximum)),
            }],
        })
    }
}

#[cfg(test)]
#[path = "random_integer_tests.rs"]
mod tests;
