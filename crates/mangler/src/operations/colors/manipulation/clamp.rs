//! Color clamp operation.
//!
//! Clamps the RGB channels of a color to a user-specified [min, max] range.
//! The alpha channel is passed through unchanged.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that clamps the RGB channels of a color to a specified min/max range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationClamp {}

impl OpColorManipulationClamp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clamp".to_string(),
            description: "Clamps the RGB channels of a color to a specified min/max range.".to_string(),
        }
    }

    /// Creates the input definitions: a color and min/max sliders.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new(
                "min".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
            Input::new(
                "max".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
        ]
    }

    /// Creates the single output definition for the clamped color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the clamp operation, constraining each RGB channel to [min, max].
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let min_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let max_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // Return early on conversion errors
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(min) = min_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max) = max_converted.unwrap() else { unreachable!() };

        // Clamp each RGB channel; alpha is preserved as-is
        let result = Color::from_srgb_float(
            color.r.clamp(min, max),
            color.g.clamp(min, max),
            color.b.clamp(min, max),
            color.a,
        );

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
    async fn test_clamp_high_values() {
        // Values above max should be pulled down to max
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpColorManipulationClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.r - 0.5).abs() < 1e-5, "Expected r=0.5, got {}", c.r);
                assert!((c.g - 0.5).abs() < 1e-5, "Expected g=0.5, got {}", c.g);
                assert!((c.b - 0.5).abs() < 1e-5, "Expected b=0.5, got {}", c.b);
                // Alpha should be unchanged
                assert!((c.a - 1.0).abs() < 1e-5, "Expected a=1.0, got {}", c.a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_clamp_low_values() {
        // Values below min should be pulled up to min
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("min".to_string(), Value::Decimal(0.3), None, None),
            Input::new("max".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpColorManipulationClamp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.r - 0.3).abs() < 1e-5, "Expected r=0.3, got {}", c.r);
                assert!((c.g - 0.3).abs() < 1e-5, "Expected g=0.3, got {}", c.g);
                assert!((c.b - 0.3).abs() < 1e-5, "Expected b=0.3, got {}", c.b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorManipulationClamp::settings();
        assert_eq!(s.name, "clamp");
        assert_eq!(OpColorManipulationClamp::create_inputs().len(), 3);
        assert_eq!(OpColorManipulationClamp::create_outputs().len(), 1);
    }
}
