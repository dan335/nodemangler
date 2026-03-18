//! Color grayscale operation.
//!
//! Converts a color to grayscale using the BT.709 relative luminance formula
//! applied in linear RGB space, then converts back to sRGB gamma.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that converts a color to grayscale using the BT.709 luminance formula.
/// Outputs both the grayscale color and the raw linear luminance value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationGrayscale {}

impl OpColorManipulationGrayscale {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grayscale".to_string(),
            description: "Converts a color to grayscale using the BT.709 relative luminance formula.".to_string(),
        }
    }

    /// Creates the single input definition: the color to convert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: the grayscale color and the linear luminance scalar.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
            Output::new("luminance".to_string(), Value::Decimal(0.0), None),
        ]
    }

    /// Executes the grayscale conversion, computing BT.709 luminance in linear RGB space.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert input
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap value
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // Convert to linear RGB for perceptually correct luminance calculation
        let (r_lin, g_lin, b_lin, alpha) = color.to_rgb_linear();

        // BT.709 relative luminance coefficients
        let luminance = (0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin).clamp(0.0, 1.0);

        // Convert linear luminance back to sRGB gamma (approximate gamma 2.2)
        let srgb = luminance.powf(1.0 / 2.2);

        let gray_color = Color::from_srgb_float(srgb, srgb, srgb, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(gray_color) },
                OutputResponse { value: Value::Decimal(luminance) },
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
    async fn test_grayscale_white_is_white() {
        // White in sRGB should remain white after grayscale conversion
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        ];
        let result = OpColorManipulationGrayscale::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                assert!((c.r - 1.0).abs() < 1e-3, "Expected r≈1.0, got {}", c.r);
                assert!((c.g - 1.0).abs() < 1e-3, "Expected g≈1.0, got {}", c.g);
                assert!((c.b - 1.0).abs() < 1e-3, "Expected b≈1.0, got {}", c.b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_grayscale_black_is_black() {
        // Black should remain black after grayscale conversion
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        ];
        let result = OpColorManipulationGrayscale::run(&mut inputs).await.unwrap();
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
        let s = OpColorManipulationGrayscale::settings();
        assert_eq!(s.name, "grayscale");
        assert_eq!(OpColorManipulationGrayscale::create_inputs().len(), 1);
        assert_eq!(OpColorManipulationGrayscale::create_outputs().len(), 2);
    }

    #[tokio::test]
    async fn test_grayscale_outputs_luminance() {
        // Verify that two output responses are produced (color + luminance)
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
        ];
        let result = OpColorManipulationGrayscale::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 2, "Expected 2 output responses");
        match &result.responses[1].value {
            Value::Decimal(_) => {}
            other => panic!("Expected Decimal for luminance output, got {:?}", other),
        }
    }
}
