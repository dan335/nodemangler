//! HWB color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into hue (degrees), whiteness,
//! blackness, and alpha.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into HWB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputHwb {}

impl OpColorOutputHwb {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to hwb".to_string(),
            description: "Converts a color to the HWB color space.".to_string(),
            help: "Splits the color into HWB: H is the hue angle in degrees (0..360), W is whiteness and B is blackness (each 0..1). Alpha is forwarded unchanged.\n\nWhiteness is the minimum RGB channel and blackness is one minus the maximum, so HWB describes how much white and black are mixed into a pure hue.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into HWB channels."),
        ]
    }

    /// Creates the output definitions: hue, whiteness, blackness, and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hue".to_string(), Value::Decimal(0.0), None)
                .with_description("Hue angle in degrees (0..360)."),
            Output::new("whiteness".to_string(), Value::Decimal(0.0), None)
                .with_description("Amount of white mixed into the hue (0..1)."),
            Output::new("blackness".to_string(), Value::Decimal(0.0), None)
                .with_description("Amount of black mixed into the hue (0..1)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to HWB float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (h, w, b, alpha) = color.to_hwb();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(w)},
                OutputResponse {value: Value::Decimal(b)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_hwb_tests.rs"]
mod tests;
