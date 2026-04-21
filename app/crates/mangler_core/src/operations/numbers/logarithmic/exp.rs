//! Exponential function (`e^x`) operation for the node graph.
//!
//! Computes Euler's number raised to the given power. Uses f64 intermediate
//! precision for accuracy, then casts back to f32.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes `e^x` (the exponential function).
///
/// Input is converted to decimal. Uses f64 for the computation to maintain
/// precision, then casts back to f32.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathExp {}

impl OpNumberMathExp {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "exp".to_string(),
            description: "Computes e raised to a power.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input defaulting to 1.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the exponential: computes `e^input`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = (input as f64).exp() as f32;

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "exp_tests.rs"]
mod tests;
