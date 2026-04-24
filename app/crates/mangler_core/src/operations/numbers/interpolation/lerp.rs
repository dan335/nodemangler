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
            help: "Computes a + (b - a) * t, blending from a toward b as t moves from 0 to 1.\n\nThe factor t is not clamped: values outside [0, 1] extrapolate past the endpoints, which is useful for easing overshoots or projecting beyond a range. All three inputs are coerced to decimal before the blend.".to_string(),
        }
    }

    /// Creates the default input list: "a" (0.0), "b" (1.0), and "t" (0.5).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Start value; returned when t is 0."),
            Input::new("b".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("End value; returned when t is 1."),
            Input::new("t".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Blend factor; 0 gives a, 1 gives b, values outside [0,1] extrapolate."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Linearly interpolated value: a + (b - a) * t.")
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
