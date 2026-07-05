//! Wrap (modulo range) operation for the node graph.
//!
//! Wraps a value into the half-open range `[min, max)` using true modulo,
//! correctly handling values below `min` and negative inputs.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that wraps a value into `[min, max)`.
///
/// All inputs are converted to decimal. A non-positive range collapses the
/// output to `min` to avoid a divide-by-zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathWrap {}

impl OpNumberMathWrap {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "wrap".to_string(),
            description: "Wraps a value into the range [min, max).".to_string(),
            help: "Performs a true modulo-wrap of the input into the half-open range [min, max), so 1.2 wraps to 0.2 and -0.3 wraps to 0.7 within [0, 1). Unlike clamp it repeats rather than saturating, and unlike the modulus node it handles negative inputs and arbitrary min offsets correctly.\n\nIf max is less than or equal to min the range is invalid and the output collapses to min.".to_string(),
        }
    }

    /// Creates the default input list: `value` (0.0), `min` (0.0), `max` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("value".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to wrap into the range."),
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Lower bound of the wrap range (inclusive)."),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Upper bound of the wrap range (exclusive)."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Input value wrapped into [min, max).")
        ]
    }

    /// Executes the wrap operation: folds `value` into `[min, max)` by modulo.
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
            min + (t - (t / range).floor() * range)
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
#[path = "wrap_tests.rs"]
mod tests;
