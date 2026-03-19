//! Color grayscale operation.
//!
//! Converts a color to grayscale using the BT.709 relative luminance formula
//! applied in linear RGB space, then converts back to sRGB gamma.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that converts a color to grayscale using the BT.709 luminance formula.
/// Outputs both the grayscale color and the raw linear luminance value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationGrayscale {}

impl OpColorManipulationGrayscale {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grayscale".to_string(),
            description: "Converts a color to grayscale using the BT.709 relative luminance formula.".to_string(),
        }
    }

    /// Creates the single input definition: the color to convert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: the grayscale color and the linear luminance scalar.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
            Output::new("luminance".to_string(), Value::Decimal(0.0), None),
        ]
    }

    /// Executes the grayscale conversion, computing BT.709 luminance in linear RGB space.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert input
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap value
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // Convert to linear RGB for perceptually correct luminance calculation
        let (r_lin, g_lin, b_lin, alpha) = color.to_rgb_linear();

        // BT.709 relative luminance coefficients
        let luminance = (0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin).clamp(0.0, 1.0);

        // Convert linear luminance back to sRGB gamma (approximate gamma 2.2)
        let srgb = luminance.powf(1.0 / 2.2);

        let gray_color = Color::from_srgb_float(srgb, srgb, srgb, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(gray_color) },
                OutputResponse { value: Value::Decimal(luminance) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "grayscale_tests.rs"]
mod tests;
