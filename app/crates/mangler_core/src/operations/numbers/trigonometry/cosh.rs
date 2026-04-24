//! Hyperbolic cosine operation for the node graph.
//!
//! Computes the hyperbolic cosine of a value.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the hyperbolic cosine of a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigCosh {}

impl OpNumberTrigCosh {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cosh".to_string(),
            description: "Computes the hyperbolic cosine of a value.".to_string(),
            help: "Returns (e^x + e^-x) / 2, the shape traced by a hanging chain (catenary). Output is always >= 1 and is symmetric: cosh(x) == cosh(-x).\n\nGrows exponentially for large |x| and can overflow f32 to infinity around |x| ~ 89. Useful for building U-shaped falloffs and hyperbolic geometry.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to take the hyperbolic cosine of."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Hyperbolic cosine of the input, always >= 1.")
        ]
    }

    /// Executes the hyperbolic cosine operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.cosh();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "cosh_tests.rs"]
mod tests;
