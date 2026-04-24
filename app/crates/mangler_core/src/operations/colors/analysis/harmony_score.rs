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
            help: "Extracts HSL hue from each color and measures the shortest angular distance between them (0-180 degrees). Five Gaussian peaks at 0, 30, 120, 150, and 180 degrees score each classical harmony relationship; the output is the single highest peak rather than a sum, so only the closest harmony wins.\n\nSaturation and lightness are ignored, so two neutral grays still score as monochromatic. The peaks have different amplitudes (complementary at 0.95, split-complementary at 0.75) to reflect how strongly each relationship is perceived.".to_string(),
        }
    }

    /// Creates the two input definitions: colors `a` and `b`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None)
                .with_description("First color whose hue is compared against the second."),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Second color whose hue is compared against the first."),
        ]
    }

    /// Creates the single output definition: score (0.0–1.0).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("score".to_string(), Value::Decimal(0.0), None)
                .with_description("Harmony score (0–1); higher near monochromatic, analogous, triadic, or complementary hues."),
        ]
    }

    /// Executes the harmony scoring between colors `a` and `b`.
    ///
    /// 1. Computes the minimum angular hue distance (0–180°) between the two colors.
    /// 2. Evaluates Gaussian peaks at each known harmonious angle.
    /// 3. Returns the maximum Gaussian value (not the sum) clamped to 0–1.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert both color inputs.
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

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
#[path = "harmony_score_tests.rs"]
mod tests;
