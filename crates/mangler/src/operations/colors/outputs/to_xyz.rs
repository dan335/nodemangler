//! CIE XYZ color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into X, Y, Z tristimulus
//! values and alpha.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CIE XYZ tristimulus values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputXyz {}

impl OpColorOutputXyz {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to xyz".to_string(),
            description: "Converts a color to the XYZ color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: X, Y, Z, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.5), None),
            Output::new("y".to_string(), Value::Decimal(0.5), None),
            Output::new("z".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to CIE XYZ float channels.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (x, y, z, alpha) = color.to_xyz();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(x)},
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(z)},
                OutputResponse {value: Value::Decimal(alpha)},
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
    async fn test_to_xyz() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
        let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
    }

    #[tokio::test]
    async fn test_to_xyz_settings() {
        let s = OpColorOutputXyz::settings();
        assert_eq!(s.name, "to xyz");
        assert_eq!(OpColorOutputXyz::create_inputs().len(), 1);
        assert_eq!(OpColorOutputXyz::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_to_xyz_black() {
        let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
        let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        // XYZ of black should all be ~0
        for i in 0..3 {
            match &result.responses[i].value {
                Value::Decimal(v) => assert!((*v).abs() < 0.01, "black XYZ[{}] should be ~0, got {}", i, v),
                other => panic!("Expected Decimal, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_to_xyz_white_y() {
        // Y of D65 white should be ~1.0
        let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
        let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
        match &result.responses[1].value {
            Value::Decimal(y) => assert!((*y - 1.0).abs() < 0.02, "white Y should be ~1, got {}", y),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_xyz_alpha_passthrough() {
        let mut inputs = color_input(0.5, 0.5, 0.5, 0.5);
        let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
        match &result.responses[3].value {
            Value::Decimal(a) => assert!((*a - 0.5).abs() < 0.01, "alpha should round trip, got {}", a),
            other => panic!("Expected Decimal for alpha, got {:?}", other),
        }
    }
}
