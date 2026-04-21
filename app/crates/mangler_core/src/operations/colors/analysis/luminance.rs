//! Color luminance operation.
//!
//! Computes the BT.709 relative luminance of a color. Relative luminance
//! is computed from linear RGB channels using the standard BT.709 coefficients
//! and is the perceptual brightness of the color normalized to [0, 1].

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the BT.709 relative luminance of a color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisLuminance {}

impl OpColorAnalysisLuminance {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "luminance".to_string(),
            description: "Computes the BT.709 relative luminance of a color.".to_string(),
        }
    }

    /// Creates the input definitions: a single color to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definition: luminance as a decimal in [0, 1].
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("luminance".to_string(), Value::Decimal(0.0), None),
        ]
    }

    /// Executes the luminance computation using BT.709 coefficients on linear RGB.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert the input color.
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early if input conversion failed.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted value.
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // Convert to linear RGB for physically accurate luminance weighting.
        // to_rgb_linear() returns (r_lin, g_lin, b_lin, alpha).
        let lin = color.to_rgb_linear();

        // BT.709 relative luminance: weighted sum of linearised RGB channels.
        let luminance = (0.2126 * lin.0 + 0.7152 * lin.1 + 0.0722 * lin.2).clamp(0.0, 1.0);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(luminance) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "luminance_tests.rs"]
mod tests;
