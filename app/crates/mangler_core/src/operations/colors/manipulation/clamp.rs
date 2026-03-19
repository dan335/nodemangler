//! Color clamp operation.
//!
//! Clamps the RGB channels of a color to a user-specified [min, max] range.
//! The alpha channel is passed through unchanged.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that clamps the RGB channels of a color to a specified min/max range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationClamp {}

impl OpColorManipulationClamp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clamp".to_string(),
            description: "Clamps the RGB channels of a color to a specified min/max range.".to_string(),
        }
    }

    /// Creates the input definitions: a color and min/max sliders.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new(
                "min".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
            Input::new(
                "max".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
        ]
    }

    /// Creates the single output definition for the clamped color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the clamp operation, constraining each RGB channel to [min, max].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let min_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let max_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(min) = min_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max) = max_converted.unwrap() else { unreachable!() };

        // Clamp each RGB channel; alpha is preserved as-is
        let result = Color::from_srgb_float(
            color.r.clamp(min, max),
            color.g.clamp(min, max),
            color.b.clamp(min, max),
            color.a,
        );

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "clamp_tests.rs"]
mod tests;
