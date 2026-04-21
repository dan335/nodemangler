//! Lerp (linear interpolation) operation for the node graph.
//!
//! Linearly interpolates between two values `a` and `b` using a factor `t`.
//! When `t = 0` the result is `a`, when `t = 1` the result is `b`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that linearly interpolates between two values.
///
/// All inputs are converted to decimal. Computes `a + (b - a) * t`.
/// The factor `t` is not clamped, allowing extrapolation beyond `[0, 1]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLerp {}

impl OpNumberMathLerp {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lerp".to_string(),
            description: "Linearly interpolates between two values.".to_string(),
        }
    }

    /// Creates the default input list: "a" (0.0), "b" (1.0), and "t" (0.5).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("t".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the lerp operation: computes `a + (b - a) * t`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let a_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let b_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let t_val = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(a) = a_val.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_val.unwrap() else { unreachable!() };
        let Value::Decimal(t) = t_val.unwrap() else { unreachable!() };

        // run node
        let value = Value::Decimal(a + (b - a) * t);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "lerp_tests.rs"]
mod tests;
