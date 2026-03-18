//! Cast-to-color operation for the node graph.
//!
//! Converts a value (bool, integer, or decimal) to a grayscale color using
//! `try_convert_to`. This provides an explicit cast node for generating colors
//! from scalar values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to a color.
///
/// Uses `Value::try_convert_to(ValueType::Color)` for the conversion.
/// Accepts booleans (black/white), integers (grayscale 0–255), and decimals
/// (grayscale 0.0–1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorCastToColor {}

impl OpColorCastToColor {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to color".to_string(),
            description: "Converts a value to a grayscale color.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input (0.0–1.0 grayscale).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), None, None),
        ]
    }

    /// Creates the default output list: a single color output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the cast: converts the input to a Color via `try_convert_to`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let result = inputs[0].value.try_convert_to(ValueType::Color);

        match result {
            Ok(color_value) => Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: color_value }],
            }),
            Err(_) => Err(OperationError {
                input_errors: vec![(0, "Unable to convert to color.".to_string())],
                node_error: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_to_color_settings() {
        let s = OpColorCastToColor::settings();
        assert_eq!(s.name, "to color");
        assert_eq!(OpColorCastToColor::create_inputs().len(), 1);
        assert_eq!(OpColorCastToColor::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_to_color_from_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, a) = c.to_srgb_float();
                assert!((r - 0.5).abs() < 0.01);
                assert!((g - 0.5).abs() < 0.01);
                assert!((b - 0.5).abs() < 0.01);
                assert!((a - 1.0).abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(255), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, _, _, _) = c.to_srgb_float();
                assert!((r - 1.0).abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_from_integer_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!(r.abs() < 0.01);
                assert!(g.abs() < 0.01);
                assert!(b.abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_from_bool_true() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!((r - 1.0).abs() < 0.01);
                assert!((g - 1.0).abs() < 0.01);
                assert!((b - 1.0).abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_from_bool_false() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!(r.abs() < 0.01);
                assert!(g.abs() < 0.01);
                assert!(b.abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_from_decimal_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!(r.abs() < 0.01);
                assert!(g.abs() < 0.01);
                assert!(b.abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_from_decimal_one() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!((r - 1.0).abs() < 0.01);
                assert!((g - 1.0).abs() < 0.01);
                assert!((b - 1.0).abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_color_passthrough() {
        let color = Color::from_srgb_float(0.2, 0.4, 0.6, 0.8);
        let mut inputs = vec![Input::new("input".to_string(), Value::Color(color), None, None)];
        let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, a) = c.to_srgb_float();
                assert!((r - 0.2).abs() < 0.01);
                assert!((g - 0.4).abs() < 0.01);
                assert!((b - 0.6).abs() < 0.01);
                assert!((a - 0.8).abs() < 0.01);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }
}
