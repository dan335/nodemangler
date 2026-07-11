//! Least common multiple (LCM) operation for the node graph.
//!
//! Computes `lcm(a, b) = |a * b| / gcd(a, b)`. Returns 0 if either input is 0.
//! Uses i64 intermediate multiplication to avoid overflow for large i32 inputs.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the least common multiple of two integers.
///
/// Both inputs are converted to integers. If either is zero, the result is 0.
/// Internally uses i64 for the product to avoid overflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLcm {}

impl OpNumberMathLcm {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lcm".to_string(),
            description: "Computes the least common multiple.".to_string(),
            help: "Computes the least common multiple of two integers as |a * b| / gcd(a, b). If either input is zero, the result is 0 by convention.\n\nThe multiplication is performed in i64 before casting back to i32 to avoid overflow for moderately large factors. Useful for finding common periods when syncing repeating patterns.".to_string(),
        }
    }

    /// Creates the default input list: two integer drag-value inputs (a=4, b=6).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(4), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("First integer operand; zero causes the result to be 0."),
            Input::new("b".to_string(), Value::Integer(6), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Second integer operand; zero causes the result to be 0."),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Least common multiple of a and b, computed as |a*b| / gcd(a,b).")
        ]
    }

    /// Computes GCD using the Euclidean algorithm (helper for LCM computation).
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

    /// Executes the LCM computation: `|a * b| / gcd(a, b)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Integer(b) = b_converted.unwrap() else { unreachable!() };

        let result = if a == 0 || b == 0 {
            0
        } else {
            // Keep the *division* in i64 as well as the product: casting the
            // product down to i32 before dividing by gcd (the old bug) makes
            // the i64 widening pointless, since the wraparound already
            // happened before the division could shrink the value back down
            // — e.g. lcm(65536, 65536) computes fine as i64 (4294967296 /
            // 65536 = 65536) but wraps to garbage if cast to i32 first.
            // Saturate the final result into i32's range since that's the
            // node's output type; lcm is always non-negative, so only the
            // upper bound can realistically be hit.
            let gcd = Self::gcd(a, b) as i64;
            let lcm = ((a as i64) * (b as i64)).abs() / gcd;
            lcm.clamp(i32::MIN as i64, i32::MAX as i64) as i32
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
#[path = "lcm_tests.rs"]
mod tests;
