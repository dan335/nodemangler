//! Snap (quantize) operation for the node graph.
//!
//! Rounds a value to the nearest multiple of a step size.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that snaps a value to the nearest multiple of a step.
///
/// Both inputs are converted to decimal. A step of zero passes the value
/// through unchanged to avoid a divide-by-zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathSnap {}

impl OpNumberMathSnap {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "snap".to_string(),
            description: "Snaps a value to the nearest multiple of a step.".to_string(),
            help: "Quantizes the input to the nearest multiple of step using (value / step).round() * step. A step of 5 maps 12 to 10 and 13 to 15; a step of 0.25 rounds to quarter increments.\n\nUseful for grid alignment, colour banding, and stepped animation. When step is 0 the value passes through unchanged so the node never divides by zero.".to_string(),
        }
    }

    /// Creates the default input list: `value` (0.0) and `step` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("value".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to snap to the nearest step multiple."),
            Input::new("step".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Step size; the value is rounded to the nearest multiple of this."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Input value rounded to the nearest multiple of step.")
        ]
    }

    /// Executes the snap operation: quantizes `value` to the nearest multiple of `step`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let value_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let step_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(value) = value_converted.unwrap() else { unreachable!() };
        let Value::Decimal(step) = step_converted.unwrap() else { unreachable!() };

        let output = if step == 0.0 { value } else { (value / step).round() * step };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "snap_tests.rs"]
mod tests;
