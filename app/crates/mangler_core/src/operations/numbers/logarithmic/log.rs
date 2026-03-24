//! Logarithm operation with arbitrary base for the node graph.
//!
//! Computes `log_base(input)`. Returns an error if the input is not positive
//! or if the base is not positive or equals 1.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the logarithm of a number with a given base.
///
/// Both input and base must be positive, and the base must not equal 1.
/// Uses f64 precision internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLog {}

impl OpNumberMathLog {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "log".to_string(),
            description: "Computes logarithm with a given base.".to_string(),
        }
    }

    /// Creates the default input list: `input` (100.0) and `base` (10.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(100.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("base".to_string(), Value::Decimal(10.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the log: computes `log_base(input)`, validating both input and base.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let base_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };
        let Value::Decimal(base) = base_converted.unwrap() else { unreachable!() };

        if input <= 0.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Input must be greater than 0.".to_string()) });
        }
        if base <= 0.0 || base == 1.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Base must be greater than 0 and not equal to 1.".to_string()) });
        }

        let result = (input as f64).log(base as f64) as f32;

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod tests;
