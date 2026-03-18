//! Color invert operation.
//!
//! Inverts the RGB channels of a color by computing `1.0 - channel` for each
//! of the red, green, and blue components. Optionally inverts the alpha channel too.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that inverts the RGB channels of a color (1.0 - channel).
/// Optionally also inverts the alpha channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationInvert {}

impl OpColorManipulationInvert {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "invert".to_string(),
            description: "Inverts the RGB channels of a color (1.0 - channel). Optionally also inverts the alpha channel.".to_string(),
        }
    }

    /// Creates the input definitions: a color and an invert-alpha toggle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new("invert alpha".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the single output definition for the inverted color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the invert operation, flipping each RGB channel and optionally the alpha.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let invert_alpha_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Bool(invert_alpha) = invert_alpha_converted.unwrap() else { unreachable!() };

        // Invert each RGB channel; conditionally invert alpha
        let new_r = 1.0 - color.r;
        let new_g = 1.0 - color.g;
        let new_b = 1.0 - color.b;
        let new_a = if invert_alpha { 1.0 - color.a } else { color.a };

        let result = Color::from_srgb_float(new_r, new_g, new_b, new_a);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(result),
            }],
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
    async fn test_invert_black_gives_white() {
        // Black (r=0, g=0, b=0, a=1) should become white (r=1, g=1, b=1, a=1)
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("invert alpha".to_string(), Value::Bool(false), None, None),
        ];
        let result = OpColorManipulationInvert::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.r - 1.0).abs() < 1e-5, "Expected r=1.0, got {}", c.r);
                assert!((c.g - 1.0).abs() < 1e-5, "Expected g=1.0, got {}", c.g);
                assert!((c.b - 1.0).abs() < 1e-5, "Expected b=1.0, got {}", c.b);
                assert!((c.a - 1.0).abs() < 1e-5, "Expected a=1.0, got {}", c.a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_invert_alpha() {
        // With invert_alpha=true, alpha 0.5 should remain 0.5 (1.0 - 0.5 = 0.5).
        // RGB: r=0 → 1, g=0 → 1, b=0 → 1. Alpha: 0.5 → 0.5.
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 0.5)), None, None),
            Input::new("invert alpha".to_string(), Value::Bool(true), None, None),
        ];
        let result = OpColorManipulationInvert::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.r - 1.0).abs() < 1e-5, "Expected r=1.0, got {}", c.r);
                assert!((c.g - 1.0).abs() < 1e-5, "Expected g=1.0, got {}", c.g);
                assert!((c.b - 1.0).abs() < 1e-5, "Expected b=1.0, got {}", c.b);
                // 1.0 - 0.5 = 0.5
                assert!((c.a - 0.5).abs() < 1e-5, "Expected a=0.5, got {}", c.a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorManipulationInvert::settings();
        assert_eq!(s.name, "invert");
        assert_eq!(OpColorManipulationInvert::create_inputs().len(), 2);
        assert_eq!(OpColorManipulationInvert::create_outputs().len(), 1);
    }
}
