//! CMYK color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into cyan, magenta, yellow,
//! key (black), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CMYK (Cyan, Magenta, Yellow, Key) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputCmyk {}

impl OpColorOutputCmyk {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to cmyk".to_string(),
            description: "Converts a color to the CMYK color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: cyan, magenta, yellow, key (black), and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("cyan".to_string(), Value::Decimal(0.5), None),
            Output::new("magenta".to_string(), Value::Decimal(0.5), None),
            Output::new("yellow".to_string(), Value::Decimal(0.5), None),
            Output::new("key (black)".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to CMYK float channels.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // run node
        let (c, m, y, k, a) = color.to_cmyk();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(c)},
                OutputResponse {value: Value::Decimal(m)},
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(k)},
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
        vec![Input::new(
            "input".to_string(),
            Value::Color(Color::from_srgb_float(r, g, b, a)),
            None, None,
        )]
    }

    #[tokio::test]
    async fn test_to_cmyk() {
        let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 5);
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_cmyk_settings() {
        let s = OpColorOutputCmyk::settings();
        assert_eq!(s.name, "to cmyk");
        assert_eq!(OpColorOutputCmyk::create_inputs().len(), 1);
        assert_eq!(OpColorOutputCmyk::create_outputs().len(), 5);
    }

    #[tokio::test]
    async fn test_to_cmyk_black() {
        let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 5);
        // K (black key) should be ~1
        match &result.responses[3].value {
            Value::Decimal(k) => assert!((*k - 1.0).abs() < 0.02, "black K should be ~1, got {}", k),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_cmyk_white() {
        let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
        let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
        // All CMY and K should be ~0 for white
        for i in 0..4 {
            match &result.responses[i].value {
                Value::Decimal(v) => assert!((*v).abs() < 0.02, "white CMYK[{}] should be ~0, got {}", i, v),
                other => panic!("Expected Decimal, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_to_cmyk_alpha_passthrough() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 0.9);
        let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
        match &result.responses[4].value {
            Value::Decimal(a) => assert!((*a - 0.9).abs() < 0.01, "alpha should round trip, got {}", a),
            other => panic!("Expected Decimal for alpha, got {:?}", other),
        }
    }
}
