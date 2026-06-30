//! Oklab color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into Oklab lightness (L),
//! green-red axis (a), blue-yellow axis (b), and alpha.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into Oklab channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputOklab {}

impl OpColorOutputOklab {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to oklab".to_string(),
            description: "Converts a color to the Oklab color space.".to_string(),
            help: "Splits the color into Oklab: L is perceptual lightness (0..1), a is the green-red axis, and b is the blue-yellow axis (each roughly -0.4..0.4). Alpha is forwarded unchanged.\n\nOklab is perceptually uniform, so it is a good space for measuring color difference and for driving gradients and mixes.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into Oklab channels."),
        ]
    }

    /// Creates the output definitions: lightness, green-red (a), blue-yellow (b), and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("lightness".to_string(), Value::Decimal(0.0), None)
                .with_description("Oklab L: perceptual lightness (0..1)."),
            Output::new("green - red".to_string(), Value::Decimal(0.0), None)
                .with_description("Oklab a axis: negative toward green, positive toward red."),
            Output::new("blue - yellow".to_string(), Value::Decimal(0.0), None)
                .with_description("Oklab b axis: negative toward blue, positive toward yellow."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to Oklab float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (l, a, b, alpha) = color.to_oklab();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(a)},
                OutputResponse {value: Value::Decimal(b)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_oklab_tests.rs"]
mod tests;
