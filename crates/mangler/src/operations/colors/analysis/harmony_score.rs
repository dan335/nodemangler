//! Color harmony score operation.
//!
//! Scores how harmoniously two colors relate on a 0.0–1.0 scale by computing
//! Gaussian peaks at known harmonious hue intervals (monochromatic, analogous,
//! triadic, split-complementary, complementary) and returning the maximum match.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that scores the harmonic relationship between two colors (0.0–1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisHarmonyScore {}

/// A Gaussian harmonic peak definition: the target angle, peak amplitude, and sigma width.
struct HarmonyPeak {
    /// The harmonious hue angle in degrees (0–180).
    center: f32,
    /// The peak score when the delta exactly matches this angle.
    peak: f32,
    /// The width (standard deviation) of the Gaussian in degrees.
    sigma: f32,
}

impl OpColorAnalysisHarmonyScore {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "harmony score".to_string(),
            description: "Scores the harmonic relationship between two colors (0–1) using Gaussian peaks at monochromatic (0°), analogous (30°), triadic (120°), split-complementary (150°), and complementary (180°) angles.".to_string(),
        }
    }

    /// Creates the two input definitions: colors `a` and `b`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the single output definition: score (0.0–1.0).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("score".to_string(), Value::Decimal(0.0), None),
        ]
    }

    /// Executes the harmony scoring between colors `a` and `b`.
    ///
    /// 1. Computes the minimum angular hue distance (0–180°) between the two colors.
    /// 2. Evaluates Gaussian peaks at each known harmonious angle.
    /// 3. Returns the maximum Gaussian value (not the sum) clamped to 0–1.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert both color inputs.
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted values.
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };

        // Extract hue from both colors via HSL.
        let (h_a, _, _, _) = a.to_hsl();
        let (h_b, _, _, _) = b.to_hsl();

        // Compute the minimum circular angular distance (always 0–180°).
        let raw_diff = (h_a - h_b).abs() % 360.0;
        let delta = if raw_diff > 180.0 { 360.0 - raw_diff } else { raw_diff };

        // Define harmonious peaks with their centers (0–180°), amplitudes, and widths.
        let peaks = [
            HarmonyPeak { center:   0.0, peak: 0.90, sigma: 10.0 }, // monochromatic
            HarmonyPeak { center:  30.0, peak: 0.80, sigma: 15.0 }, // analogous
            HarmonyPeak { center: 120.0, peak: 0.85, sigma: 12.0 }, // triadic
            HarmonyPeak { center: 150.0, peak: 0.75, sigma: 12.0 }, // split-complementary
            HarmonyPeak { center: 180.0, peak: 0.95, sigma: 10.0 }, // complementary
        ];

        // Evaluate each Gaussian: peak * exp(-0.5 * ((delta - center) / sigma)^2).
        // The final score is the maximum of all evaluations.
        let score = peaks.iter()
            .map(|p| {
                let normalized = (delta - p.center) / p.sigma;
                p.peak * (-0.5 * normalized * normalized).exp()
            })
            .fold(0.0_f32, f32::max)
            .clamp(0.0, 1.0);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(score) },
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

    fn pair_inputs(a: Color, b: Color) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(a), None, None),
            Input::new("b".to_string(), Value::Color(b), None, None),
        ]
    }

    #[tokio::test]
    async fn test_same_hue_monochromatic_peak() {
        // Two colors with the same hue → 0° delta → monochromatic peak ~0.9.
        let color_a = Color::from_hsl(120.0, 0.8, 0.3, 1.0);
        let color_b = Color::from_hsl(120.0, 0.6, 0.7, 1.0);
        let mut inputs = pair_inputs(color_a, color_b);
        let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();

        let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!(
            (score - 0.9).abs() < 0.05,
            "Same hue should score near monochromatic peak 0.9, got {}",
            score
        );
    }

    #[tokio::test]
    async fn test_complementary_180_peak() {
        // Colors 180° apart → complementary peak ~0.95.
        let color_a = Color::from_hsl(0.0, 1.0, 0.5, 1.0);
        let color_b = Color::from_hsl(180.0, 1.0, 0.5, 1.0);
        let mut inputs = pair_inputs(color_a, color_b);
        let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();

        let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!(
            (score - 0.95).abs() < 0.05,
            "180° apart should score near complementary peak 0.95, got {}",
            score
        );
    }

    #[tokio::test]
    async fn test_90_degrees_low_score() {
        // Colors ~90° apart sit between peaks and should have a noticeably lower score.
        let color_a = Color::from_hsl(0.0, 1.0, 0.5, 1.0);
        let color_b = Color::from_hsl(90.0, 1.0, 0.5, 1.0);
        let mut inputs = pair_inputs(color_a, color_b);
        let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();

        let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
        // 90° is equidistant from triadic (120°) and analogous (30°); score should be relatively low.
        assert!(score < 0.5, "90° apart should have a low harmony score, got {}", score);
    }

    #[tokio::test]
    async fn test_score_clamped_0_to_1() {
        // Score must always be within 0–1.
        for (h_a, h_b) in [(0.0, 0.0), (0.0, 45.0), (0.0, 90.0), (0.0, 135.0), (0.0, 180.0)] {
            let ca = Color::from_hsl(h_a, 1.0, 0.5, 1.0);
            let cb = Color::from_hsl(h_b, 1.0, 0.5, 1.0);
            let mut inputs = pair_inputs(ca, cb);
            let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();
            let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
            assert!(score >= 0.0 && score <= 1.0, "Score out of range: {}", score);
        }
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorAnalysisHarmonyScore::settings();
        assert_eq!(s.name, "harmony score");
        assert_eq!(OpColorAnalysisHarmonyScore::create_inputs().len(), 2);
        assert_eq!(OpColorAnalysisHarmonyScore::create_outputs().len(), 1);
    }
}
