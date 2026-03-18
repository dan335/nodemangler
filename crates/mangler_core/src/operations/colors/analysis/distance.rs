//! Color distance operation.
//!
//! Computes the CIE76 Delta E perceptual distance and Euclidean RGB distance
//! between two colors. Delta E measures perceptual difference in Lab color space,
//! while RGB distance measures raw channel difference in sRGB space.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes CIE76 Delta E and Euclidean RGB distance between two colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisDistance {}

impl OpColorAnalysisDistance {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "distance".to_string(),
            description: "Computes the CIE76 Delta E perceptual distance and Euclidean RGB distance between two colors.".to_string(),
        }
    }

    /// Creates the input definitions: two colors (a, b) to compare.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: Delta E (perceptual) and RGB (Euclidean) distances.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("delta_e".to_string(), Value::Decimal(0.0), None),
            Output::new("rgb_distance".to_string(), Value::Decimal(0.0), None),
        ]
    }

    /// Executes the distance computation, returning CIE76 Delta E and Euclidean RGB distance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs to their expected types.
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);

        // Return early if any input failed conversion.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted values.
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };

        // Convert both colors to Lab space for CIE76 Delta E computation.
        // to_lab() returns (L: 0-100, a: -128..128, b: -128..128, alpha: 0-1).
        let la = a.to_lab();
        let lb = b.to_lab();

        // CIE76 Delta E: Euclidean distance in Lab color space.
        let delta_e = ((lb.0 - la.0).powi(2) + (lb.1 - la.1).powi(2) + (lb.2 - la.2).powi(2)).sqrt();

        // Euclidean RGB distance: straight-line distance in sRGB channel space.
        let rgb_distance = ((b.r - a.r).powi(2) + (b.g - a.g).powi(2) + (b.b - a.b).powi(2)).sqrt();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(delta_e) },
                OutputResponse { value: Value::Decimal(rgb_distance) },
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

    fn distance_inputs(a: Color, b: Color) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(a), None, None),
            Input::new("b".to_string(), Value::Color(b), None, None),
        ]
    }

    #[tokio::test]
    async fn test_same_color_distance_is_zero() {
        // Comparing a color with itself should yield zero for both distance metrics.
        let color = Color::from_srgb_float(0.5, 0.3, 0.8, 1.0);
        let mut inputs = distance_inputs(color, color);
        let result = OpColorAnalysisDistance::run(&mut inputs).await.unwrap();

        let Value::Decimal(delta_e) = result.responses[0].value else { panic!("Expected Decimal") };
        let Value::Decimal(rgb_dist) = result.responses[1].value else { panic!("Expected Decimal") };

        assert!(delta_e.abs() < 0.001, "same color delta_e should be zero, got {}", delta_e);
        assert!(rgb_dist.abs() < 0.001, "same color rgb_distance should be zero, got {}", rgb_dist);
    }

    #[tokio::test]
    async fn test_black_white_distance() {
        // Black and white are maximally different in both Lab and RGB spaces.
        let black = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let mut inputs = distance_inputs(black, white);
        let result = OpColorAnalysisDistance::run(&mut inputs).await.unwrap();

        let Value::Decimal(delta_e) = result.responses[0].value else { panic!("Expected Decimal") };
        let Value::Decimal(rgb_dist) = result.responses[1].value else { panic!("Expected Decimal") };

        // Delta E between black and white in Lab is ~100 (L goes from 0 to 100).
        assert!(delta_e > 50.0, "black-white delta_e should be large, got {}", delta_e);

        // RGB distance between (0,0,0) and (1,1,1) is sqrt(3) ≈ 1.732.
        assert!((rgb_dist - 3.0_f32.sqrt()).abs() < 0.01, "black-white rgb_distance should be ~1.732, got {}", rgb_dist);
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorAnalysisDistance::settings();
        assert_eq!(s.name, "distance");
        assert_eq!(OpColorAnalysisDistance::create_inputs().len(), 2);
        assert_eq!(OpColorAnalysisDistance::create_outputs().len(), 2);
    }
}
