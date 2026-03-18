//! Set alpha operation.
//!
//! Replaces or multiplies the alpha channel of a color. In replace mode the
//! alpha is set directly to the provided value; in multiply mode the existing
//! alpha is multiplied by the provided value.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that replaces or multiplies the alpha channel of a color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationSetAlpha {}

impl OpColorManipulationSetAlpha {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "set alpha".to_string(),
            description: "Replaces or multiplies the alpha channel of a color.".to_string(),
        }
    }

    /// Creates the input definitions: a color, an alpha value slider, and a multiply toggle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new(
                "alpha".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
            Input::new("multiply".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the single output definition for the color with modified alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the set-alpha operation, either replacing or multiplying the alpha channel.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let alpha_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let multiply_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);

        // Return early on conversion errors
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };
        let Value::Bool(multiply) = multiply_converted.unwrap() else { unreachable!() };

        // Either multiply by existing alpha or replace it outright
        let new_alpha = if multiply { color.a * alpha } else { alpha };

        let result = Color::from_srgb_float(color.r, color.g, color.b, new_alpha);

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
    async fn test_set_alpha_replace() {
        // Replace mode: alpha should be exactly the provided value regardless of original
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.5, 0.25, 0.8)), None, None),
            Input::new("alpha".to_string(), Value::Decimal(0.4), None, None),
            Input::new("multiply".to_string(), Value::Bool(false), None, None),
        ];
        let result = OpColorManipulationSetAlpha::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.a - 0.4).abs() < 1e-5, "Expected a=0.4, got {}", c.a);
                // RGB channels should be unchanged
                assert!((c.r - 1.0).abs() < 1e-5, "Expected r=1.0, got {}", c.r);
                assert!((c.g - 0.5).abs() < 1e-5, "Expected g=0.5, got {}", c.g);
                assert!((c.b - 0.25).abs() < 1e-5, "Expected b=0.25, got {}", c.b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_set_alpha_multiply() {
        // Multiply mode: new alpha = original_alpha * alpha_input
        // 0.8 * 0.5 = 0.4
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.5, 0.25, 0.8)), None, None),
            Input::new("alpha".to_string(), Value::Decimal(0.5), None, None),
            Input::new("multiply".to_string(), Value::Bool(true), None, None),
        ];
        let result = OpColorManipulationSetAlpha::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.a - 0.4).abs() < 1e-5, "Expected a=0.4 (0.8*0.5), got {}", c.a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorManipulationSetAlpha::settings();
        assert_eq!(s.name, "set alpha");
        assert_eq!(OpColorManipulationSetAlpha::create_inputs().len(), 3);
        assert_eq!(OpColorManipulationSetAlpha::create_outputs().len(), 1);
    }
}
