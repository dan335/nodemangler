//! Greatest common divisor (GCD) operation for the node graph.
//!
//! Computes the GCD of two integers using the Euclidean algorithm. Negative
//! inputs are handled by taking their absolute value. Returns 0 when both
//! inputs are 0.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the greatest common divisor of two integers.
///
/// Uses the Euclidean algorithm with absolute values. Both inputs are converted
/// to integers. If both inputs are zero, the result is 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathGcd {}

impl OpNumberMathGcd {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gcd".to_string(),
            description: "Computes the greatest common divisor.".to_string(),
        }
    }

    /// Creates the default input list: two integer drag-value inputs (a=12, b=8).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(12), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Integer(8), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Computes GCD using the Euclidean algorithm on absolute values.
    fn gcd(a: i32, b: i32) -> i32 {
        let mut a = a.abs();
        let mut b = b.abs();
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }

    /// Executes the GCD computation on the two integer inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Integer(b) = b_converted.unwrap() else { unreachable!() };

        let result = if a == 0 && b == 0 {
            0
        } else {
            Self::gcd(a, b)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "gcd_tests.rs"]
mod tests;
