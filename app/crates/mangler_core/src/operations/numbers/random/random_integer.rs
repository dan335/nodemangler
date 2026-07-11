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
            help: "Rolls a uniformly distributed integer in the half-open range [min, max) each time the generate trigger fires, using the fastrand PRNG.\n\nmin is inclusive and max is exclusive. If max is not strictly greater than min, it is automatically clamped to min + 1 so the range is always valid (and the output equals min). The generator is non-cryptographic and not seeded per-graph, so results are not reproducible.".to_string(),
        }
    }

    /// Creates the default input list: trigger, `min` (i32::MIN), and `max` (i32::MAX).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None)
                .with_description("Trigger that causes the node to roll a new random integer."),
            Input::new("min".to_string(), Value::Integer(i32::MIN), None, None)
                .with_description("Inclusive lower bound of the random range."),
            Input::new("max".to_string(), Value::Integer(i32::MAX), None, None)
                .with_description("Exclusive upper bound; clamped to min+1 if not greater than min."),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Uniformly distributed random integer in [min, max).")
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

        // When minimum is i32::MAX, saturating_add(1) can't push maximum past
        // it, so maximum ends up equal to minimum: an empty exclusive range
        // `MAX..MAX` that `fastrand::i32` would panic on. Treat an
        // empty/degenerate range as "no room to roll" and just return
        // minimum, same as the min==max case already covered by the
        // saturating_add clamp above for every other value.
        let value = if maximum <= minimum {
            minimum
        } else {
            fastrand::i32(minimum..maximum)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(value),
            }],
        })
    }
}

#[cfg(test)]
#[path = "random_integer_tests.rs"]
mod tests;
