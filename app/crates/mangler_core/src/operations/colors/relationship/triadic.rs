//! Triadic color harmony operation.
//!
//! Generates two triadic harmony colors at +120° and +240° hue offsets
//! from the input color, producing an evenly-spaced three-color triad.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Rotates a hue value (0–360) by a given number of degrees, wrapping correctly.
fn rotate_hue(h: f32, degrees: f32) -> f32 {
    ((h + degrees) % 360.0 + 360.0) % 360.0
}

/// Operation that generates two triadic harmony colors at +120° and +240° hue offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyTriadic {}

impl OpColorHarmonyTriadic {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "triadic".to_string(),
            description: "Generates two triadic harmony colors at +120° and +240° hue offsets.".to_string(),
        }
    }

    /// Creates the single input definition: the source color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the two output definitions: triadic_a (+120°) and triadic_b (+240°).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("triadic_a".to_string(), Value::Color(Color::default()), None),
            Output::new("triadic_b".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the triadic harmony, producing colors at +120° and +240° hue offsets.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert input
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap value
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // Decompose into HSL components for hue rotation
        let (h, s, l, a) = color.to_hsl();

        // Triadic colors divide the hue wheel into three equal 120° segments
        let triadic_a = Color::from_hsl(rotate_hue(h, 120.0), s, l, a);
        let triadic_b = Color::from_hsl(rotate_hue(h, 240.0), s, l, a);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(triadic_a) },
                OutputResponse { value: Value::Color(triadic_b) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "triadic_tests.rs"]
mod tests;
