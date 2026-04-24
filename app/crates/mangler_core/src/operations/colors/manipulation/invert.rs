//! Color invert operation.
//!
//! Inverts the RGB channels of a color by computing `1.0 - channel` for each
//! of the red, green, and blue components. Optionally inverts the alpha channel too.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that inverts the RGB channels of a color (1.0 - channel).
/// Optionally also inverts the alpha channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationInvert {}

impl OpColorManipulationInvert {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "invert".to_string(),
            description: "Inverts the RGB channels of a color (1.0 - channel). Optionally also inverts the alpha channel.".to_string(),
            help: "Produces a photographic-style negative by replacing each sRGB channel with 1 - channel. Black becomes white, red becomes cyan, and so on.\n\nThe operation runs on the gamma-encoded sRGB floats; inverting in linear light would give slightly different midtones, so use this for color-picker style inversion. When 'invert alpha' is on, fully opaque becomes fully transparent and vice versa; otherwise alpha is preserved.".to_string(),
        }
    }

    /// Creates the input definitions: a color and an invert-alpha toggle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color whose RGB channels will be inverted (1 - channel)."),
            Input::new("invert alpha".to_string(), Value::Bool(false), None, None)
                .with_description("When true the alpha channel is also inverted; otherwise it passes through."),
        ]
    }

    /// Creates the single output definition for the inverted color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Inverted color with each channel replaced by 1 - channel."),
        ]
    }

    /// Executes the invert operation, flipping each RGB channel and optionally the alpha.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let invert_alpha_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Bool(invert_alpha) = invert_alpha_converted.unwrap() else { unreachable!() };

        // Invert each RGB channel; conditionally invert alpha
        let new_r = 1.0 - color.r;
        let new_g = 1.0 - color.g;
        let new_b = 1.0 - color.b;
        let new_a = if invert_alpha { 1.0 - color.a } else { color.a };

        let result = Color::from_srgb_float(new_r, new_g, new_b, new_a);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "invert_tests.rs"]
mod tests;
