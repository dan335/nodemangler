//! Select (multiplexer) operation.
//!
//! Chooses between two values based on a boolean condition. When the condition
//! is `true`, the "if true" input is forwarded; otherwise, the "if false" input
//! is forwarded. The output value retains its original type (no coercion is
//! applied to the selected branch).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Select (mux) node that picks between two values based on a boolean condition.
///
/// Acts as a ternary operator: `condition ? if_true : if_false`. The condition
/// input is coerced to boolean, but the two branch inputs are passed through
/// without type conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicFlowSelect {}

impl OpLogicFlowSelect {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "select".to_string(),
            description: "Selects between two values based on a condition.".to_string(),
        }
    }

    /// Creates the default inputs: a boolean condition, and two decimal branch values ("if true" and "if false").
    pub fn create_inputs() -> Vec<Input> {
        let mut if_true = Input::new("if true".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None);
        if_true.accepts_any_type = true;

        let mut if_false = Input::new("if false".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None);
        if_false.accepts_any_type = true;

        vec![
            Input::new("condition".to_string(), Value::Bool(false), None, None),
            if_true,
            if_false,
        ]
    }

    /// Creates the default output: a single decimal output defaulting to 0.0.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Evaluates the condition and forwards the appropriate branch value.
    ///
    /// Only the condition input is coerced (to boolean). The selected branch
    /// value is cloned and output as-is, preserving its original type.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let condition_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(condition) = condition_converted.unwrap() else { unreachable!() };

        // Forward the selected branch value without any type coercion
        let value = if condition {
            inputs[1].value.clone() // "if true" branch
        } else {
            inputs[2].value.clone() // "if false" branch
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value }],
        })
    }
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
