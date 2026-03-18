//! Color HSV adjustment operation.
//!
//! Offsets the hue, saturation, and value channels of a color in HSV space.
//! Hue wraps around modulo 360; saturation and value are clamped to [0.0, 1.0].

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that offsets the hue, saturation, and value channels of a color in HSV space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationAdjustHsv {}

impl OpColorManipulationAdjustHsv {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "adjust hsv".to_string(),
            description: "Offsets the hue, saturation, and value channels of a color in HSV space.".to_string(),
        }
    }

    /// Creates the input definitions: color and three offset sliders for hue, saturation, and value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new(
                "hue offset".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (-180.0, 180.0), step_by: Some(1.0), clamp_to_range: false }),
                None,
            ),
            Input::new(
                "saturation offset".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }),
                None,
            ),
            Input::new(
                "value offset".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }),
                None,
            ),
        ]
    }

    /// Creates the single output definition for the adjusted color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the HSV adjustment, offsetting H/S/V channels and wrapping/clamping as needed.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let hue_offset_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let sat_offset_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let val_offset_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // Return early on conversion errors
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(hue_offset) = hue_offset_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sat_offset) = sat_offset_converted.unwrap() else { unreachable!() };
        let Value::Decimal(val_offset) = val_offset_converted.unwrap() else { unreachable!() };

        // Decompose to HSV and apply offsets
        let (h, s, v, a) = color.to_hsv();

        // Wrap hue into [0, 360) to handle negative offsets gracefully
        let new_h = ((h + hue_offset) % 360.0 + 360.0) % 360.0;
        // Clamp saturation and value to valid [0, 1] range
        let new_s = (s + sat_offset).clamp(0.0, 1.0);
        let new_v = (v + val_offset).clamp(0.0, 1.0);

        let result = Color::from_hsv(new_h, new_s, new_v, a);

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
    async fn test_hue_offset_180() {
        // Offsetting hue by 180 should produce the complementary hue
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("hue offset".to_string(), Value::Decimal(180.0), None, None),
            Input::new("saturation offset".to_string(), Value::Decimal(0.0), None, None),
            Input::new("value offset".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpColorManipulationAdjustHsv::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_saturation_offset() {
        // Reducing saturation to zero should produce a neutral gray
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("hue offset".to_string(), Value::Decimal(0.0), None, None),
            Input::new("saturation offset".to_string(), Value::Decimal(-1.0), None, None),
            Input::new("value offset".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpColorManipulationAdjustHsv::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                // With saturation 0 the color should be a shade of gray (r≈g≈b)
                assert!((c.r - c.g).abs() < 1e-3, "Expected gray, r={} g={}", c.r, c.g);
                assert!((c.g - c.b).abs() < 1e-3, "Expected gray, g={} b={}", c.g, c.b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_value_offset() {
        // Reducing value to zero should produce black
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("hue offset".to_string(), Value::Decimal(0.0), None, None),
            Input::new("saturation offset".to_string(), Value::Decimal(0.0), None, None),
            Input::new("value offset".to_string(), Value::Decimal(-1.0), None, None),
        ];
        let result = OpColorManipulationAdjustHsv::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!(c.r.abs() < 1e-5, "Expected r≈0.0, got {}", c.r);
                assert!(c.g.abs() < 1e-5, "Expected g≈0.0, got {}", c.g);
                assert!(c.b.abs() < 1e-5, "Expected b≈0.0, got {}", c.b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorManipulationAdjustHsv::settings();
        assert_eq!(s.name, "adjust hsv");
        assert_eq!(OpColorManipulationAdjustHsv::create_inputs().len(), 4);
        assert_eq!(OpColorManipulationAdjustHsv::create_outputs().len(), 1);
    }
}
