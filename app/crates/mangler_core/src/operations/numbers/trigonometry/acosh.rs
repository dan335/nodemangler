//! Inverse hyperbolic cosine operation for the node graph.
//!
//! Computes the inverse hyperbolic cosine (area cosh) of a value. Defined only
//! for inputs greater than or equal to 1; smaller inputs raise an error.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the inverse hyperbolic cosine of a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAcosh {}

impl OpNumberTrigAcosh {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "acosh".to_string(),
            description: "Computes the inverse hyperbolic cosine of a value.".to_string(),
            help: "Returns acosh(input), the inverse of cosh. Defined only for inputs greater than or equal to 1, so cosh maps [1, inf) to [0, inf).\n\nInputs below 1 raise an error rather than returning NaN, so problems surface immediately. Pair with cosh for the forward direction.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value (>= 1) to take the inverse hyperbolic cosine of; below 1 errors."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Inverse hyperbolic cosine of the input.")
        ]
    }

    /// Executes the inverse hyperbolic cosine, erroring for inputs below 1.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let c = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(val) = c.unwrap() else { unreachable!() };

        if val < 1.0 {
            return Err(OperationError {
                input_errors: vec![(0, "acosh is undefined for inputs below 1.".to_string())],
                node_error: None,
            });
        }

        let result = val.acosh();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "acosh_tests.rs"]
mod tests;
