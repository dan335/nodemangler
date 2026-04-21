//! Smoothstep operation for the node graph.
//!
//! Performs smooth Hermite interpolation between two edge values. The result is
//! clamped to `[0, 1]` and follows the standard GLSL `smoothstep` formula:
//! `t * t * (3 - 2t)` where `t = clamp((x - edge0) / (edge1 - edge0), 0, 1)`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes smooth Hermite interpolation between two edges.
///
/// All inputs are converted to decimal. Returns an error if `edge0 == edge1`
/// (degenerate range). The output is always in the range `[0.0, 1.0]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathSmoothstep {}

impl OpNumberMathSmoothstep {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "smoothstep".to_string(),
            description: "Smooth Hermite interpolation between two edges.".to_string(),
        }
    }

    /// Creates the default input list: "input" (0.5), "edge0" (0.0), and "edge1" (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("edge0".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("edge1".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the smoothstep operation.
    ///
    /// Computes `t = clamp((input - edge0) / (edge1 - edge0), 0, 1)` then
    /// returns `t * t * (3 - 2t)`. Returns an error if `edge0 == edge1`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let input_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let edge0_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let edge1_val = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(input) = input_val.unwrap() else { unreachable!() };
        let Value::Decimal(edge0) = edge0_val.unwrap() else { unreachable!() };
        let Value::Decimal(edge1) = edge1_val.unwrap() else { unreachable!() };

        // validate edges are different
        if edge0 == edge1 {
            return Err(OperationError {
                input_errors: vec![], node_error: Some("edge0 and edge1 must be different.".to_string()),
            });
        }

        // run node: smoothstep formula
        let t = ((input - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        let value = Value::Decimal(t * t * (3.0 - 2.0 * t));

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "smoothstep_tests.rs"]
mod tests;
