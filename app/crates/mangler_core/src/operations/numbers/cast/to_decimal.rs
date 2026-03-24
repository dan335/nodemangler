//! Cast-to-decimal operation for the node graph.
//!
//! Converts a numeric value to a decimal (f32) using `try_convert_to`.
//! Integer inputs are widened; decimal inputs pass through unchanged.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to decimal (f32).
///
/// Uses `Value::try_convert_to(ValueType::Decimal)` for the conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCastToDecimal {}

impl OpNumberCastToDecimal {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to decimal".to_string(),
            description: "Converts a number to a decimal.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(f32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the cast: converts the input to a decimal via `try_convert_to`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(n)) = inputs[0].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to decimal.".to_string())], node_error: None })};

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(n),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_decimal_tests.rs"]
mod tests;
