//! Hypotenuse operation for the node graph.
//!
//! Computes `sqrt(a^2 + b^2)`, the length of a 2D vector, using the
//! numerically stable `f32::hypot`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the hypotenuse of two legs.
///
/// Both inputs are converted to decimal and combined with `a.hypot(b)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathHypot {}

impl OpNumberMathHypot {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hypot".to_string(),
            description: "Computes sqrt(a^2 + b^2), the length of a 2D vector.".to_string(),
            help: "Returns the hypotenuse sqrt(a*a + b*b), i.e. the length of the 2D vector (a, b) or the distance from the origin. Legs 3 and 4 give 5.\n\nUses f32::hypot internally, which avoids the intermediate overflow or underflow you would get from squaring large or tiny values directly.".to_string(),
        }
    }

    /// Creates the default input list: `a` (3.0) and `b` (4.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(3.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("First leg of the right triangle."),
            Input::new("b".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Second leg of the right triangle."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Hypotenuse sqrt(a^2 + b^2).")
        ]
    }

    /// Executes the hypotenuse operation: computes `a.hypot(b)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_converted.unwrap() else { unreachable!() };

        let output = a.hypot(b);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "hypot_tests.rs"]
mod tests;
