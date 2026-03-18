//! Double split-complementary color harmony operation.
//!
//! Generates four harmony colors: two near the base color (+30°, -30°) and two near
//! the complementary (+150°, +210°), creating a rich six-point spread on the hue wheel.

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

/// Operation that generates four double split-complementary harmony colors.
///
/// Produces two colors adjacent to the base (+30°, -30°) and two adjacent to
/// the complement (+150°, +210°), forming a six-color harmonic palette when
/// combined with the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyDoubleSplitComplementary {}

impl OpColorHarmonyDoubleSplitComplementary {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "double split complementary".to_string(),
            description: "Generates four harmony colors at +30°, -30°, +150°, and +210° hue offsets from the input, forming a six-color double split-complementary scheme.".to_string(),
        }
    }

    /// Creates the single input definition: the source color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the four output definitions:
    /// - `split_base_a` (+30° from input)
    /// - `split_base_b` (-30° from input, i.e. +330°)
    /// - `split_comp_a` (+150° from input)
    /// - `split_comp_b` (+210° from input)
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("split_base_a".to_string(), Value::Color(Color::default()), None),
            Output::new("split_base_b".to_string(), Value::Color(Color::default()), None),
            Output::new("split_comp_a".to_string(), Value::Color(Color::default()), None),
            Output::new("split_comp_b".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the double split-complementary harmony.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert input color.
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted value.
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // Decompose into HSL components for hue rotation.
        let (h, s, l, a) = color.to_hsl();

        // split_base_a sits +30° from the input hue (adjacent on the positive side).
        let split_base_a = Color::from_hsl(rotate_hue(h, 30.0), s, l, a);
        // split_base_b sits -30° from the input hue (adjacent on the negative side, i.e. +330°).
        let split_base_b = Color::from_hsl(rotate_hue(h, -30.0), s, l, a);
        // split_comp_a sits +150° away, flanking the complementary on the positive side.
        let split_comp_a = Color::from_hsl(rotate_hue(h, 150.0), s, l, a);
        // split_comp_b sits +210° away, flanking the complementary on the negative side.
        let split_comp_b = Color::from_hsl(rotate_hue(h, 210.0), s, l, a);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(split_base_a) },
                OutputResponse { value: Value::Color(split_base_b) },
                OutputResponse { value: Value::Color(split_comp_a) },
                OutputResponse { value: Value::Color(split_comp_b) },
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
    async fn test_double_split_complementary_red() {
        // Red hue ~0° → split_base_a ~30°, split_base_b ~330°, split_comp_a ~150°, split_comp_b ~210°.
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
        ];
        let result = OpColorHarmonyDoubleSplitComplementary::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4, "Expected 4 output responses");

        // Check split_base_a hue (~30°).
        let Value::Color(sba) = &result.responses[0].value else { panic!("Expected Color") };
        let (h_sba, _, _, _) = sba.to_hsl();
        assert!((h_sba - 30.0).abs() < 1.0, "Expected split_base_a hue ~30°, got {}", h_sba);

        // Check split_base_b hue (~330°).
        let Value::Color(sbb) = &result.responses[1].value else { panic!("Expected Color") };
        let (h_sbb, _, _, _) = sbb.to_hsl();
        assert!((h_sbb - 330.0).abs() < 1.0, "Expected split_base_b hue ~330°, got {}", h_sbb);

        // Check split_comp_a hue (~150°).
        let Value::Color(sca) = &result.responses[2].value else { panic!("Expected Color") };
        let (h_sca, _, _, _) = sca.to_hsl();
        assert!((h_sca - 150.0).abs() < 1.0, "Expected split_comp_a hue ~150°, got {}", h_sca);

        // Check split_comp_b hue (~210°).
        let Value::Color(scb) = &result.responses[3].value else { panic!("Expected Color") };
        let (h_scb, _, _, _) = scb.to_hsl();
        assert!((h_scb - 210.0).abs() < 1.0, "Expected split_comp_b hue ~210°, got {}", h_scb);
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorHarmonyDoubleSplitComplementary::settings();
        assert_eq!(s.name, "double split complementary");
        assert_eq!(OpColorHarmonyDoubleSplitComplementary::create_inputs().len(), 1);
        assert_eq!(OpColorHarmonyDoubleSplitComplementary::create_outputs().len(), 4);
    }
}
