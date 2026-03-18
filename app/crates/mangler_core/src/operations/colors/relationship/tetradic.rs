//! Tetradic color harmony operation.
//!
//! Generates three tetradic harmony colors at +90°, +180°, and +270° hue offsets
//! from the input color, forming a four-color rectangle on the hue wheel.

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

/// Operation that generates three tetradic harmony colors at +90°, +180°, and +270° hue offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyTetradic {}

impl OpColorHarmonyTetradic {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tetradic".to_string(),
            description: "Generates three tetradic harmony colors at +90°, +180°, and +270° hue offsets, forming a four-color rectangle on the hue wheel.".to_string(),
        }
    }

    /// Creates the single input definition: the source color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the three output definitions: tetradic_b (+90°), tetradic_c (+180°), tetradic_d (+270°).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("tetradic_b".to_string(), Value::Color(Color::default()), None),
            Output::new("tetradic_c".to_string(), Value::Color(Color::default()), None),
            Output::new("tetradic_d".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the tetradic harmony, producing colors at +90°, +180°, and +270° hue offsets.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert input color.
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted value.
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

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
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_tetradic_red() {
        // Red hue ~0° should produce tetradic_b ~90°, tetradic_c ~180°, tetradic_d ~270°.
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
        ];
        let result = OpColorHarmonyTetradic::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3, "Expected 3 output responses");

        // Check tetradic_b hue (~90°).
        let Value::Color(tb) = &result.responses[0].value else { panic!("Expected Color") };
        let (h_tb, _, _, _) = tb.to_hsl();
        assert!((h_tb - 90.0).abs() < 1.0, "Expected tetradic_b hue ~90°, got {}", h_tb);

        // Check tetradic_c hue (~180°).
        let Value::Color(tc) = &result.responses[1].value else { panic!("Expected Color") };
        let (h_tc, _, _, _) = tc.to_hsl();
        assert!((h_tc - 180.0).abs() < 1.0, "Expected tetradic_c hue ~180°, got {}", h_tc);

        // Check tetradic_d hue (~270°).
        let Value::Color(td) = &result.responses[2].value else { panic!("Expected Color") };
        let (h_td, _, _, _) = td.to_hsl();
        assert!((h_td - 270.0).abs() < 1.0, "Expected tetradic_d hue ~270°, got {}", h_td);
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorHarmonyTetradic::settings();
        assert_eq!(s.name, "tetradic");
        assert_eq!(OpColorHarmonyTetradic::create_inputs().len(), 1);
        assert_eq!(OpColorHarmonyTetradic::create_outputs().len(), 3);
    }
}
