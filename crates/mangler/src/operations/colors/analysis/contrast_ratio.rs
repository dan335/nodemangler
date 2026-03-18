//! WCAG contrast ratio operation.
//!
//! Computes the WCAG 2.1 contrast ratio between two colors and evaluates
//! whether the pair meets AA (minimum 4.5:1) and AAA (enhanced 7:1)
//! accessibility compliance thresholds for normal-sized text.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the WCAG contrast ratio and AA/AAA compliance between two colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisContrastRatio {}

impl OpColorAnalysisContrastRatio {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "contrast ratio".to_string(),
            description: "Computes the WCAG contrast ratio between two colors and checks AA (4.5:1) and AAA (7:1) compliance.".to_string(),
        }
    }

    /// Creates the input definitions: two colors (a, b) to compare.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: ratio, AA pass, and AAA pass.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("ratio".to_string(), Value::Decimal(0.0), None),
            Output::new("passes_AA".to_string(), Value::Bool(false), None),
            Output::new("passes_AAA".to_string(), Value::Bool(false), None),
        ]
    }

    /// Executes the WCAG contrast ratio computation between two colors.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert both color inputs.
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);

        // Return early if any input failed conversion.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted values.
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };

        // WCAG relative luminance using BT.709 coefficients on linear RGB channels.
        let relative_luminance = |c: &Color| -> f32 {
            let lin = c.to_rgb_linear();
            (0.2126 * lin.0 + 0.7152 * lin.1 + 0.0722 * lin.2).clamp(0.0, 1.0)
        };

        // l1 is the lighter color (higher luminance), l2 is the darker one.
        let l1 = relative_luminance(&a).max(relative_luminance(&b));
        let l2 = relative_luminance(&a).min(relative_luminance(&b));

        // WCAG contrast ratio formula: (L1 + 0.05) / (L2 + 0.05).
        // Result is in [1, 21] where 1 = identical colors, 21 = black on white.
        let ratio = (l1 + 0.05) / (l2 + 0.05);

        // WCAG AA requires a minimum ratio of 4.5:1 for normal text.
        let passes_aa = ratio >= 4.5;
        // WCAG AAA requires a minimum ratio of 7:1 for enhanced contrast.
        let passes_aaa = ratio >= 7.0;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(ratio) },
                OutputResponse { value: Value::Bool(passes_aa) },
                OutputResponse { value: Value::Bool(passes_aaa) },
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

    fn contrast_inputs(a: Color, b: Color) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(a), None, None),
            Input::new("b".to_string(), Value::Color(b), None, None),
        ]
    }

    #[tokio::test]
    async fn test_black_white_contrast() {
        // Black on white is the maximum possible contrast, approximately 21:1.
        let black = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let mut inputs = contrast_inputs(black, white);
        let result = OpColorAnalysisContrastRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        let Value::Bool(passes_aa) = result.responses[1].value else { panic!("Expected Bool") };
        let Value::Bool(passes_aaa) = result.responses[2].value else { panic!("Expected Bool") };

        assert!(
            (ratio - 21.0).abs() < 0.1,
            "black-white contrast ratio should be ~21.0, got {}",
            ratio
        );
        assert!(passes_aa, "black-white should pass AA");
        assert!(passes_aaa, "black-white should pass AAA");
    }

    #[tokio::test]
    async fn test_same_color_contrast() {
        // Comparing a color with itself gives a ratio of exactly 1.0, failing all thresholds.
        let color = Color::from_srgb_float(0.4, 0.6, 0.2, 1.0);
        let mut inputs = contrast_inputs(color, color);
        let result = OpColorAnalysisContrastRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        let Value::Bool(passes_aa) = result.responses[1].value else { panic!("Expected Bool") };
        let Value::Bool(passes_aaa) = result.responses[2].value else { panic!("Expected Bool") };

        assert!(
            (ratio - 1.0).abs() < 0.001,
            "same color contrast ratio should be 1.0, got {}",
            ratio
        );
        assert!(!passes_aa, "same color should not pass AA");
        assert!(!passes_aaa, "same color should not pass AAA");
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorAnalysisContrastRatio::settings();
        assert_eq!(s.name, "contrast ratio");
        assert_eq!(OpColorAnalysisContrastRatio::create_inputs().len(), 2);
        assert_eq!(OpColorAnalysisContrastRatio::create_outputs().len(), 3);
    }
}
