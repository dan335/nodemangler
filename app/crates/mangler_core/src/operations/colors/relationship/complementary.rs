//! Complementary and split-complementary color harmony operation.
//!
//! Generates the complementary color at 180° and the split-complementary
//! pair at 150° and 210° hue offsets from the input color.

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

/// Operation that generates the complementary color (180°) and split-complementary
/// pair (150°, 210°) from an input color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyComplementary {}

impl OpColorHarmonyComplementary {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "complementary".to_string(),
            description: "Generates the complementary color (180°) and split-complementary pair (150°, 210°) from an input color.".to_string(),
        }
    }

    /// Creates the single input definition: the source color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the three output definitions: complementary, split_a, and split_b.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("complementary".to_string(), Value::Color(Color::default()), None),
            Output::new("split_a".to_string(), Value::Color(Color::default()), None),
            Output::new("split_b".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the complementary harmony, producing colors at 180°, 150°, and 210° offsets.
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

        // Complementary color is directly opposite on the hue wheel
        let complementary = Color::from_hsl(rotate_hue(h, 180.0), s, l, a);
        // Split-complementary colors flank the complement at ±30° (150° and 210°)
        let split_a = Color::from_hsl(rotate_hue(h, 150.0), s, l, a);
        let split_b = Color::from_hsl(rotate_hue(h, 210.0), s, l, a);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(complementary) },
                OutputResponse { value: Value::Color(split_a) },
                OutputResponse { value: Value::Color(split_b) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "complementary_tests.rs"]
mod tests;
