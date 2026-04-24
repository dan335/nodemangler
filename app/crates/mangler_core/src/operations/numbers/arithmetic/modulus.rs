//! Modulus (remainder) operation for the node graph.
//!
//! Computes `a % n` for integer and decimal types. Returns an error if `n` is zero.
//! Uses Rust's remainder semantics (result has the sign of the dividend).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the remainder of `a` divided by `n`.
///
/// The divisor `n` is converted to decimal for validation. Returns an error
/// when `n` is zero. The result sign matches the dividend (Rust `%` semantics).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathModulus {}

impl OpNumberMathModulus {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "modulus".to_string(),
            description: "Returns the remainder after dividing two numbers.".to_string(),
            help: "Computes a % n using Rust's remainder semantics, where the sign of the result matches the sign of the dividend (e.g. -7 % 3 = -1, not 2).\n\nSetting n to zero raises a division-by-zero error. When a is an integer the divisor is truncated to i32 first, so a fractional n like 0.5 will be treated as 0 and also error out. Useful for wrapping values to a range or detecting multiples.".to_string(),
        }
    }

    /// Creates the default input list: value `a` (0.5) and divisor `n` (1.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Dividend whose remainder is taken."),
            Input::new("n".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Divisor; must be non-zero, otherwise the node errors."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
                .with_description("Remainder of a divided by n; sign matches the dividend.")
        ]
    }

    /// Executes the modulus: computes `a % n`, returning an error if `n` is zero.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(n)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert 'n' to Decimal.".to_string())], node_error: None })};

        if n == 0.0 {
            return Err(OperationError {
                input_errors: vec![(1, "Division by zero.".to_string())], node_error: None,
            });
        }

        let value = match &inputs[0].value {
            Value::Integer(a) => {
                // Integer path casts `n` to i32, so fractional divisors like 0.5 truncate to 0
                // and would panic on integer %. Guard against it explicitly.
                let n_int = n as i32;
                if n_int == 0 {
                    return Err(OperationError {
                        input_errors: vec![(1, "Division by zero.".to_string())], node_error: None,
                    });
                }
                Value::Integer(*a % n_int)
            }

            Value::Decimal(a) => Value::Decimal(*a % n),

            _ => {return Err(OperationError {
                input_errors: vec![], node_error: Some("Error converting.".to_string()),
            });}
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "modulus_tests.rs"]
mod tests;
