//! CIE xyY color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from chromaticity (x, y) and
//! luminance (Y), plus alpha.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from CIE xyY channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputXyy {}

impl OpColorInputXyy {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "xyy".to_string(),
            description: "Creates a color using the CIE xyY color space.".to_string(),
            help: "Builds an sRGB color from CIE xyY: x and y are the chromaticity coordinates (which point on the chromaticity diagram), and Y is the luminance (0..1). D65 white is approximately x=0.3127, y=0.3290.\n\nxyY separates 'what color' (x, y) from 'how bright' (Y), which is handy for picking chromaticities or analyzing white points. Coordinates outside the sRGB gamut will be clipped. Alpha is passed through unchanged.".to_string(),
        }
    }

    /// Creates the input definitions: x, y chromaticity (0..1), Y luminance (0..1), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("x".to_string(), Value::Decimal(0.3127), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("CIE x chromaticity coordinate."),
            Input::new("y".to_string(), Value::Decimal(0.3290), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("CIE y chromaticity coordinate."),
            Input::new("luminance".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Y: luminance (0 = black, 1 = full)."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the CIE xyY + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from CIE xyY float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let x_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let big_y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(big_y) = big_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        let color = Color::from_xyy(x, y, big_y, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Color(color) }],
        })
    }
}

#[cfg(test)]
#[path = "xyy_tests.rs"]
mod tests;
