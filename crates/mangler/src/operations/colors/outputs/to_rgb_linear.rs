//! Linear RGB color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into its red, green, blue, and
//! alpha channel values in the linear (non-gamma-encoded) RGB color space.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into linear RGB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputRgbLinear {}

impl OpColorOutputRgbLinear {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to rgb linear".to_string(),
            description: "Converts a color to the RGB linear color space.".to_string(),
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

    /// Executes the operation, converting the input color to linear RGB float channels.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (r, g, b, a) = color.to_rgb_linear();

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
    async fn test_to_rgb_linear() {
        let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[3].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_rgb_linear_settings() {
        let s = OpColorOutputRgbLinear::settings();
        assert_eq!(s.name, "to rgb linear");
        assert_eq!(OpColorOutputRgbLinear::create_inputs().len(), 1);
        assert_eq!(OpColorOutputRgbLinear::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_to_rgb_linear_black() {
        let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(r) => assert!((*r).abs() < 0.01, "black R linear should be ~0, got {}", r),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_rgb_linear_white() {
        let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
        let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(r) => assert!((*r - 1.0).abs() < 0.01, "white R linear should be ~1, got {}", r),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_rgb_linear_alpha_passthrough() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 0.8);
        let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
        match &result.responses[3].value {
            Value::Decimal(a) => assert!((*a - 0.8).abs() < 0.01, "alpha should round trip, got {}", a),
            other => panic!("Expected Decimal for alpha, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_rgb_linear_gamma_expansion() {
        // Linear RGB of sRGB 0.5 should be less than 0.5 (gamma expansion darkens midtones)
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(r) => assert!(*r < 0.5, "linear R of sRGB 0.5 should be < 0.5 due to gamma, got {}", r),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
