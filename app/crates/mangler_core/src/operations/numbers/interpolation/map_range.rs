//! Map range operation for the node graph.
//!
//! Remaps a value from one numeric range to another. For example, mapping `0.5`
//! from `[0, 1]` to `[0, 100]` yields `50.0`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
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
            help: "Linearly rescales input from [in min, in max] onto [out min, out max] using out_min + (input - in_min) * (out_max - out_min) / (in_max - in_min).\n\nThe result is not clamped, so inputs outside the source range produce outputs outside the destination range. If in min equals in max the source range has zero width and the node errors. Inverted ranges (min > max) flip the mapping direction.".to_string(),
        }
    }

    /// Creates the default input list: "input" (0.5), "in min" (0.0), "in max" (1.0),
    /// "out min" (0.0), and "out max" (100.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to remap from the input range to the output range."),
            Input::new("in min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Lower bound of the input range."),
            Input::new("in max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Upper bound of the input range; must differ from in min."),
            Input::new("out min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Lower bound of the output range."),
            Input::new("out max".to_string(), Value::Decimal(100.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Upper bound of the output range."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Input remapped linearly from [in min, in max] onto [out min, out max].")
        ]
    }

    /// Executes the map range operation.
    ///
    /// Remaps `input` from `[in_min, in_max]` to `[out_min, out_max]`.
    /// Returns an error if `in_min == in_max` (zero-width input range).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! {
            inputs;
            Decimal(input) = 0,
            Decimal(in_min) = 1,
            Decimal(in_max) = 2,
            Decimal(out_min) = 3,
            Decimal(out_max) = 4,
        }

        // validate input range is not zero
        if in_min == in_max {
            return Err(OperationError {
                input_errors: vec![], node_error: Some("Input range must not be zero.".to_string()),
            });
        }

        // run node
        let value = Value::Decimal(out_min + (input - in_min) * (out_max - out_min) / (in_max - in_min));

        Ok(OperationResponse { 
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
