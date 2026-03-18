//! LCH color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into lightness, chroma, hue,
//! and alpha channel values. LCH is the cylindrical form of CIE L*a*b*.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into LCH (Lightness, Chroma, Hue) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputLch {}

impl OpColorOutputLch {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to lch".to_string(),
            description: "Converts a color to the LCH color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: lightness, chroma, hue, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("lightness".to_string(), Value::Decimal(0.5), None),
            Output::new("chroma".to_string(), Value::Decimal(0.5), None),
            Output::new("hue".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to LCH float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (l, c, h, a) = color.to_lch();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(c)},
                OutputResponse {value: Value::Decimal(h)},
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
    async fn test_to_lch() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
    }

    #[tokio::test]
    async fn test_to_lch_settings() {
        let s = OpColorOutputLch::settings();
        assert_eq!(s.name, "to lch");
        assert_eq!(OpColorOutputLch::create_inputs().len(), 1);
        assert_eq!(OpColorOutputLch::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_to_lch_black_lightness() {
        let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        // Lightness of black should be ~0
        match &result.responses[0].value {
            Value::Decimal(l) => assert!((*l).abs() < 0.5, "black L should be ~0, got {}", l),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_lch_grey_zero_chroma() {
        // A grey should have near-zero chroma
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
        match &result.responses[1].value {
            Value::Decimal(c) => assert!((*c).abs() < 0.05, "grey chroma should be ~0, got {}", c),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_lch_alpha_passthrough() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 0.6);
        let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
        match &result.responses[3].value {
            Value::Decimal(a) => assert!((*a - 0.6).abs() < 0.01, "alpha should round trip, got {}", a),
            other => panic!("Expected Decimal for alpha, got {:?}", other),
        }
    }
}
