//! Set alpha operation.
//!
//! Replaces or multiplies the alpha channel of a color. In replace mode the
//! alpha is set directly to the provided value; in multiply mode the existing
//! alpha is multiplied by the provided value.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that replaces or multiplies the alpha channel of a color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationSetAlpha {}

impl OpColorManipulationSetAlpha {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "set alpha".to_string(),
            description: "Replaces or multiplies the alpha channel of a color.".to_string(),
        }
    }

    /// Creates the input definitions: a color, an alpha value slider, and a multiply toggle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new(
                "alpha".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
            Input::new("multiply".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the single output definition for the color with modified alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the set-alpha operation, either replacing or multiplying the alpha channel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let alpha_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let multiply_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };
        let Value::Bool(multiply) = multiply_converted.unwrap() else { unreachable!() };

        // Either multiply by existing alpha or replace it outright
        let new_alpha = if multiply { color.a * alpha } else { alpha };

        let result = Color::from_srgb_float(color.r, color.g, color.b, new_alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "set_alpha_tests.rs"]
mod tests;
