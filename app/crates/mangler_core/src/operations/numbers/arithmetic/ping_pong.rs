//! Ping-pong (triangle fold) operation for the node graph.
//!
//! Folds a value back and forth within `[min, max]` like a bouncing ball,
//! producing a continuous triangle wave.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that ping-pongs a value within `[min, max]`.
///
/// All inputs are converted to decimal. A non-positive range collapses the
/// output to `min` to avoid a divide-by-zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathPingPong {}

impl OpNumberMathPingPong {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ping pong".to_string(),
            description: "Folds a value back and forth within [min, max].".to_string(),
            help: "Bounces the input within the range [min, max] instead of wrapping, producing a continuous triangle wave: as the value rises past max it reverses and falls back toward min, and vice versa. Feeding a steadily increasing value yields a back-and-forth oscillation.\n\nUnlike wrap there is no discontinuity at the boundaries. If max is less than or equal to min the range is invalid and the output collapses to min.".to_string(),
        }
    }

    /// Creates the default input list: `value` (0.0), `min` (0.0), `max` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("value".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to fold back and forth within the range."),
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Lower bound of the fold range."),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Upper bound of the fold range."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Input value folded into [min, max] as a triangle wave.")
        ]
    }

    /// Executes the ping-pong operation: folds `value` back and forth within `[min, max]`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let value_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let min_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let max_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(value) = value_converted.unwrap() else { unreachable!() };
        let Value::Decimal(min) = min_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max) = max_converted.unwrap() else { unreachable!() };

        let range = max - min;
        let output = if range <= 0.0 {
            min
        } else {
            let t = value - min;
            let m = t - 2.0 * range * ((t / (2.0 * range)).floor());
            let tri = if m <= range { m } else { 2.0 * range - m };
            min + tri
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "ping_pong_tests.rs"]
mod tests;
