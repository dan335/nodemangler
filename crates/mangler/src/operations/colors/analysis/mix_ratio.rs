//! Color mix ratio (reverse-lerp) operation.
//!
//! Given a source color, a target color, and a mixed color, computes the blending
//! ratio `t` such that `lerp(source, target, t) ≈ mixed`. This is a per-channel
//! reverse linear interpolation averaged across the computable RGB channels.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the blend ratio `t` from a known source, target, and mixed color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisMixRatio {}

impl OpColorAnalysisMixRatio {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mix ratio".to_string(),
            description: "Reverse-lerp: finds the blend ratio t (0–1) such that lerp(source, target, t) ≈ mixed. Averages the per-channel t values for non-degenerate channels.".to_string(),
        }
    }

    /// Creates the three input definitions: source, target, and mixed colors.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("source".to_string(), Value::Color(Color::default()), None, None),
            Input::new("target".to_string(), Value::Color(Color::default()), None, None),
            Input::new("mixed".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the single output definition: ratio (0.0–1.0).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("ratio".to_string(), Value::Decimal(0.0), None),
        ]
    }

    /// Executes the reverse-lerp computation.
    ///
    /// For each RGB channel where `|target - source| > 1e-6`, computes
    /// `t = (mixed - source) / (target - source)`. The final ratio is the
    /// average of all non-degenerate channel results, clamped to 0–1.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert all three color inputs.
        let src_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let tgt_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let mix_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted values.
        let Value::Color(source) = src_converted.unwrap() else { unreachable!() };
        let Value::Color(target) = tgt_converted.unwrap() else { unreachable!() };
        let Value::Color(mixed) = mix_converted.unwrap() else { unreachable!() };

        // Compute per-channel reverse-lerp for each of R, G, B.
        let channels = [
            (source.r, target.r, mixed.r),
            (source.g, target.g, mixed.g),
            (source.b, target.b, mixed.b),
        ];

        let mut sum_t = 0.0_f32;
        let mut count = 0_u32;

        for (s, t, m) in channels {
            let diff = t - s;
            if diff.abs() > 1e-6 {
                // Channel is non-degenerate: compute reverse lerp.
                let channel_t = (m - s) / diff;
                sum_t += channel_t;
                count += 1;
            }
            // Degenerate channels (source == target) are skipped entirely.
        }

        // If all channels are degenerate (source == target), default to 0.0.
        let ratio = if count > 0 {
            (sum_t / count as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(ratio) },
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

    fn mix_inputs(source: Color, target: Color, mixed: Color) -> Vec<Input> {
        vec![
            Input::new("source".to_string(), Value::Color(source), None, None),
            Input::new("target".to_string(), Value::Color(target), None, None),
            Input::new("mixed".to_string(), Value::Color(mixed), None, None),
        ]
    }

    #[tokio::test]
    async fn test_mixed_equals_source_returns_zero() {
        // If mixed == source, then t should be 0.0.
        let source = Color::from_srgb_float(0.2, 0.4, 0.6, 1.0);
        let target = Color::from_srgb_float(0.8, 0.6, 0.4, 1.0);
        let mut inputs = mix_inputs(source, target, source);
        let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!((ratio - 0.0).abs() < 0.001, "mixed=source should give ratio 0.0, got {}", ratio);
    }

    #[tokio::test]
    async fn test_mixed_equals_target_returns_one() {
        // If mixed == target, then t should be 1.0.
        let source = Color::from_srgb_float(0.2, 0.4, 0.6, 1.0);
        let target = Color::from_srgb_float(0.8, 0.6, 0.4, 1.0);
        let mut inputs = mix_inputs(source, target, target);
        let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!((ratio - 1.0).abs() < 0.001, "mixed=target should give ratio 1.0, got {}", ratio);
    }

    #[tokio::test]
    async fn test_midpoint_returns_half() {
        // If mixed is the exact midpoint, t should be 0.5.
        let source = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let target = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let midpoint = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0);
        let mut inputs = mix_inputs(source, target, midpoint);
        let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!((ratio - 0.5).abs() < 0.001, "midpoint should give ratio 0.5, got {}", ratio);
    }

    #[tokio::test]
    async fn test_degenerate_all_channels_same() {
        // If source == target, all channels are degenerate; ratio defaults to 0.0.
        let same = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0);
        let mut inputs = mix_inputs(same, same, same);
        let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!((ratio - 0.0).abs() < 0.001, "Degenerate source==target should give ratio 0.0, got {}", ratio);
    }

    #[tokio::test]
    async fn test_ratio_clamped_to_0_1() {
        // Even if mixed is outside the source–target range, ratio is clamped.
        let source = Color::from_srgb_float(0.3, 0.3, 0.3, 1.0);
        let target = Color::from_srgb_float(0.7, 0.7, 0.7, 1.0);
        // mixed far beyond target
        let beyond = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let mut inputs = mix_inputs(source, target, beyond);
        let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

        let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!(ratio >= 0.0 && ratio <= 1.0, "Ratio out of range: {}", ratio);
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorAnalysisMixRatio::settings();
        assert_eq!(s.name, "mix ratio");
        assert_eq!(OpColorAnalysisMixRatio::create_inputs().len(), 3);
        assert_eq!(OpColorAnalysisMixRatio::create_outputs().len(), 1);
    }
}
