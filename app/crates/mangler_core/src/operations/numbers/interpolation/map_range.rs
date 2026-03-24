//! Map range operation for the node graph.
//!
//! Remaps a value from one numeric range to another. For example, mapping `0.5`
//! from `[0, 1]` to `[0, 100]` yields `50.0`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that remaps a value from one range to another.
///
/// All inputs are converted to decimal. Returns an error if the input range
/// is zero (`in_min == in_max`). The formula is:
/// `out_min + (input - in_min) * (out_max - out_min) / (in_max - in_min)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathMapRange {}

impl OpNumberMathMapRange {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "map range".to_string(),
            description: "Remaps a value from one range to another.".to_string(),
        }
    }

    /// Creates the default input list: "input" (0.5), "in min" (0.0), "in max" (1.0),
    /// "out min" (0.0), and "out max" (100.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("in min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("in max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("out min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("out max".to_string(), Value::Decimal(100.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the map range operation.
    ///
    /// Remaps `input` from `[in_min, in_max]` to `[out_min, out_max]`.
    /// Returns an error if `in_min == in_max` (zero-width input range).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let in_min_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let in_max_val = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let out_min_val = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let out_max_val = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_val.unwrap() else { unreachable!() };
        let Value::Decimal(in_min) = in_min_val.unwrap() else { unreachable!() };
        let Value::Decimal(in_max) = in_max_val.unwrap() else { unreachable!() };
        let Value::Decimal(out_min) = out_min_val.unwrap() else { unreachable!() };
        let Value::Decimal(out_max) = out_max_val.unwrap() else { unreachable!() };

        // validate input range is not zero
        if in_min == in_max {
            return Err(OperationError {
                input_errors: vec![], node_error: Some("Input range must not be zero.".to_string()),
            });
        }

        // run node
        let value = Value::Decimal(out_min + (input - in_min) * (out_max - out_min) / (in_max - in_min));

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "map_range_tests.rs"]
mod tests;
