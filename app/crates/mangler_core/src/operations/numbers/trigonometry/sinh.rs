//! Hyperbolic sine operation for the node graph.
//!
//! Computes the hyperbolic sine of a value.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the hyperbolic sine of a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigSinh {}

impl OpNumberTrigSinh {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sinh".to_string(),
            description: "Computes the hyperbolic sine of a value.".to_string(),
            help: "Returns (e^x - e^-x) / 2, an odd function with sinh(0) == 0 and sinh(-x) == -sinh(x).\n\nGrows exponentially for large |x| and can overflow f32 to +/- infinity around |x| ~ 89. Near zero it behaves almost linearly, making it handy for gentle-to-extreme response curves.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value to take the hyperbolic sine of."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Hyperbolic sine of the input: (e^x - e^-x) / 2.")
        ]
    }

    /// Executes the hyperbolic sine operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.sinh();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "sinh_tests.rs"]
mod tests;
