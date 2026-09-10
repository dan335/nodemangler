//! Tetradic color harmony operation.
//!
//! Generates three tetradic harmony colors at +90°, +180°, and +270° hue offsets
//! from the input color, forming a four-color rectangle on the hue wheel.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Rotates a hue value (0–360) by a given number of degrees, wrapping correctly.
fn rotate_hue(h: f32, degrees: f32) -> f32 {
    ((h + degrees) % 360.0 + 360.0) % 360.0
}

/// Operation that generates three tetradic harmony colors at +90°, +180°, and +270° hue offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyTetradic {}

impl OpColorHarmonyTetradic {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tetradic".to_string(),
            description: "Generates three tetradic harmony colors at +90°, +180°, and +270° hue offsets, forming a four-color rectangle on the hue wheel.".to_string(),
            help: "Converts the input to HSL, keeps saturation and lightness, and rotates the hue by 90, 180, and 270 degrees to produce the other three corners of a square on the color wheel. Together with the input, the outputs make a four-color scheme with two complementary pairs.\n\nBecause all operations are hue rotations, grays produce identical outputs. Tetradic palettes tend to be bold; if colors feel too busy, lower saturation or use monochromatic output for the supporting roles.".to_string(),
        }
    }

    /// Creates the single input definition: the source color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Base color whose three tetradic partners are generated."),
        ]
    }

    /// Creates the three output definitions: tetradic_b (+90°), tetradic_c (+180°), tetradic_d (+270°).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("tetradic_b".to_string(), Value::Color(Color::default()), None)
                .with_description("Tetradic partner at the base hue shifted by +90°."),
            Output::new("tetradic_c".to_string(), Value::Color(Color::default()), None)
                .with_description("Tetradic partner at the base hue shifted by +180° (the complement)."),
            Output::new("tetradic_d".to_string(), Value::Color(Color::default()), None)
                .with_description("Tetradic partner at the base hue shifted by +270°."),
        ]
    }

    /// Executes the tetradic harmony, producing colors at +90°, +180°, and +270° hue offsets.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Color(color) = 0,
        }

        // Decompose into HSL components for hue rotation.
        let (h, s, l, a) = color.to_hsl();

        // Tetradic colors divide the hue wheel into four equal 90° segments.
        let tetradic_b = Color::from_hsl(rotate_hue(h, 90.0), s, l, a);
        let tetradic_c = Color::from_hsl(rotate_hue(h, 180.0), s, l, a);
        let tetradic_d = Color::from_hsl(rotate_hue(h, 270.0), s, l, a);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(tetradic_b) },
                OutputResponse { value: Value::Color(tetradic_c) },
                OutputResponse { value: Value::Color(tetradic_d) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "tetradic_tests.rs"]
mod tests;
