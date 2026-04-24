//! Cast-to-color operation for the node graph.
//!
//! Converts a value (bool, integer, or decimal) to a grayscale color using
//! `try_convert_to`. This provides an explicit cast node for generating colors
//! from scalar values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to a color.
///
/// Uses `Value::try_convert_to(ValueType::Color)` for the conversion.
/// Accepts booleans (black/white), integers (grayscale 0–255), and decimals
/// (grayscale 0.0–1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorCastToColor {}

impl OpColorCastToColor {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to color".to_string(),
            description: "Converts a value to a grayscale color.".to_string(),
            help: "Accepts a Bool, Integer, or Decimal input and packs it into the R, G, and B channels of a fully opaque color. Bool false becomes black and true becomes white. Integers are treated as 0-255 sRGB bytes and normalized; decimals are taken as 0-1 sRGB floats without further remapping.\n\nUse this as an explicit bridge when a node expects a Color and you want a gray of a specific intensity from a numeric value. The output is always fully opaque (alpha = 1).".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input (0.0–1.0 grayscale).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), None, None)
                .with_description("Scalar (bool, integer, or decimal) interpreted as a grayscale intensity."),
        ]
    }

    /// Creates the default output list: a single color output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Opaque grayscale color built from the input scalar."),
        ]
    }

    /// Executes the cast: converts the input to a Color via `try_convert_to`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let result = inputs[0].value.try_convert_to(ValueType::Color);

        match result {
            Ok(color_value) => Ok(OperationResponse { 
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: color_value }],
            }),
            Err(_) => Err(OperationError {
                input_errors: vec![(0, "Unable to convert to color.".to_string())],
                node_error: None,
            }),
        }
    }
}

#[cfg(test)]
#[path = "to_color_tests.rs"]
mod tests;
