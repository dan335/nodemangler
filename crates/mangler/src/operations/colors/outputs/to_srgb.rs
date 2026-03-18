//! sRGB color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into its red, green, blue, and
//! alpha channel values in the standard sRGB (gamma-encoded) color space.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into sRGB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputRgb {}

impl OpColorOutputRgb {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to rgb".to_string(),
            description: "Converts a color to the RGB color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: red, green, blue, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::Decimal(0.5), None),
            Output::new("green".to_string(), Value::Decimal(0.5), None),
            Output::new("blue".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to sRGB float channels.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (r, g, b, a) = color.to_srgb_float();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(r)},
                OutputResponse {value: Value::Decimal(g)},
                OutputResponse {value: Value::Decimal(b)},
                OutputResponse {value: Value::Decimal(a)},
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

    fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
        vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
    }

    #[tokio::test]
    async fn test_to_srgb() {
        let mut inputs = color_input(0.8, 0.2, 0.4, 0.5);
        let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.8).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_srgb_settings() {
        let s = OpColorOutputRgb::settings();
        assert_eq!(s.name, "to rgb");
        assert_eq!(OpColorOutputRgb::create_inputs().len(), 1);
        assert_eq!(OpColorOutputRgb::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_to_srgb_black_round_trip() {
        let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01, "black R should be ~0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_srgb_white_round_trip() {
        let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
        let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01, "white R should be ~1, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_srgb_alpha_channel() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 0.3);
        let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
        match &result.responses[3].value {
            Value::Decimal(a) => assert!((*a - 0.3).abs() < 0.01, "alpha should round trip, got {}", a),
            other => panic!("Expected Decimal for alpha, got {:?}", other),
        }
    }
}
