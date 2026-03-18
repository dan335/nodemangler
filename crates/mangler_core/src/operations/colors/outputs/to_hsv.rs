//! HSV color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into hue, saturation, value
//! (brightness), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into HSV (Hue, Saturation, Value) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputHsv {}

impl OpColorOutputHsv {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to hsv".to_string(),
            description: "Converts a color to the HSV color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: hue, saturation, value, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hue".to_string(), Value::Decimal(0.5), None),
            Output::new("saturation".to_string(), Value::Decimal(0.5), None),
            Output::new("value".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to HSV float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (h, s, v, a) = color.to_hsv();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(s)},
                OutputResponse {value: Value::Decimal(v)},
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
    async fn test_to_hsv() {
        let mut inputs = color_input(0.0, 1.0, 0.0, 1.0);
        let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 120.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_hsv_settings() {
        let s = OpColorOutputHsv::settings();
        assert_eq!(s.name, "to hsv");
        assert_eq!(OpColorOutputHsv::create_inputs().len(), 1);
        assert_eq!(OpColorOutputHsv::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_to_hsv_black() {
        let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        // Value of black should be ~0
        match &result.responses[2].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01, "black V should be ~0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_hsv_white() {
        let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
        let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
        // Value of white should be ~1, saturation ~0
        match &result.responses[2].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01, "white V should be ~1, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_hsv_alpha_passthrough() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 0.7);
        let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
        match &result.responses[3].value {
            Value::Decimal(a) => assert!((*a - 0.7).abs() < 0.01, "alpha should round trip, got {}", a),
            other => panic!("Expected Decimal for alpha, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_hsv_pure_red_hue() {
        let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
        // Pure red has hue 0 or 360
        match &result.responses[0].value {
            Value::Decimal(h) => assert!((*h).abs() < 1.0 || (*h - 360.0).abs() < 1.0, "red hue should be ~0/360, got {}", h),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
