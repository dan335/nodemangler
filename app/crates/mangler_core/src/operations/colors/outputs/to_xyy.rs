//! CIE xyY color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into chromaticity (x, y),
//! luminance (Y), and alpha.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CIE xyY channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputXyy {}

impl OpColorOutputXyy {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to xyy".to_string(),
            description: "Converts a color to the CIE xyY color space.".to_string(),
            help: "Splits the color into CIE xyY: x and y are the chromaticity coordinates and Y is the luminance (0..1). Alpha is forwarded unchanged.\n\nxyY separates chromaticity (x, y) from luminance (Y), which is convenient for chromaticity-diagram work and white-point analysis. Black has no defined chromaticity, so it reports the D65 white point (x=0.3127, y=0.3290).".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into CIE xyY channels."),
        ]
    }

    /// Creates the output definitions: x, y chromaticity, Y luminance, and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.0), None)
                .with_description("CIE x chromaticity coordinate."),
            Output::new("y".to_string(), Value::Decimal(0.0), None)
                .with_description("CIE y chromaticity coordinate."),
            Output::new("luminance".to_string(), Value::Decimal(0.0), None)
                .with_description("Y: luminance (0..1)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to CIE xyY float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (x, y, big_y, alpha) = color.to_xyy();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(x)},
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(big_y)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_xyy_tests.rs"]
mod tests;
