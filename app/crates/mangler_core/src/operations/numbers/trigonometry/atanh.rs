//! Inverse hyperbolic tangent operation for the node graph.
//!
//! Computes the inverse hyperbolic tangent (area tanh) of a value. Defined
//! only on the open interval (-1, 1); inputs at or beyond the bounds error.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the inverse hyperbolic tangent of a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAtanh {}

impl OpNumberTrigAtanh {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "atanh".to_string(),
            description: "Computes the inverse hyperbolic tangent of a value.".to_string(),
            help: "Returns atanh(input), the inverse of tanh. Defined only on the open interval (-1, 1); the magnitude approaches infinity as the input approaches ±1.\n\nInputs with |input| >= 1 raise an error rather than returning infinity or NaN, so problems surface immediately. Pair with tanh for the forward direction.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Value in (-1, 1) to take the inverse hyperbolic tangent of; |input| >= 1 errors."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Inverse hyperbolic tangent of the input.")
        ]
    }

    /// Executes the inverse hyperbolic tangent, erroring when |input| >= 1.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let c = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(val) = c.unwrap() else { unreachable!() };

        if val.abs() >= 1.0 {
            return Err(OperationError {
                input_errors: vec![(0, "atanh is undefined for |input| ≥ 1.".to_string())],
                node_error: None,
            });
        }

        let result = val.atanh();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "atanh_tests.rs"]
mod tests;
