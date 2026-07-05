//! Degrees-to-radians conversion for the node graph.
//!
//! Converts an angle in degrees to radians.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts an angle in degrees to radians.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigToRadians {}

impl OpNumberTrigToRadians {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to radians".to_string(),
            description: "Converts an angle from degrees to radians.".to_string(),
            help: "Returns input * pi / 180, converting an angle measured in degrees into radians.\n\nUse it to feed degree-based angles into the trig nodes (sin, cos, tan), which expect radians. Pair with 'to degrees' for the inverse.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("degrees".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Angle in degrees to convert to radians."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("radians".to_string(), Value::Decimal(0.0), None)
                .with_description("The input angle expressed in radians.")
        ]
    }

    /// Executes the degrees-to-radians conversion.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.to_radians();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_radians_tests.rs"]
mod tests;
