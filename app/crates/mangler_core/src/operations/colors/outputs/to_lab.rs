//! CIE L*a*b* color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into lightness (L*),
//! green-red axis (a*), blue-yellow axis (b*), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CIE L*a*b* channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputLab {}

impl OpColorOutputLab {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to lab".to_string(),
            description: "Converts a color to the LAB color space.".to_string(),
            help: "Splits the color into CIE L*a*b*: L* is perceived lightness (0-100), a* is the green-red axis (negative green, positive red, roughly -128..128), and b* is the blue-yellow axis (negative blue, positive yellow). Alpha is forwarded unchanged.\n\nThe conversion goes through linear RGB and XYZ using a D65 white point. Because Lab is perceptually uniform, equal numeric steps in its channels feel roughly equal visually, which is why Delta E color-difference math is done here.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into CIE L*a*b* channels."),
        ]
    }

    /// Creates the output definitions: lightness, green-red (a*), blue-yellow (b*), and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("lightness".to_string(), Value::Decimal(0.5), None)
                .with_description("CIE L* perceived lightness (0 = black, 100 = white)."),
            Output::new("green - red".to_string(), Value::Decimal(0.5), None)
                .with_description("CIE a* axis: negative toward green, positive toward red."),
            Output::new("blue - yellow".to_string(), Value::Decimal(0.5), None)
                .with_description("CIE b* axis: negative toward blue, positive toward yellow."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to CIE L*a*b* float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (l, a, b, alpha) = color.to_lab();

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
#[path = "to_lab_tests.rs"]
mod tests;
