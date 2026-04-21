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
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert all three color inputs.
        let src_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let tgt_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let mix_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

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
#[path = "mix_ratio_tests.rs"]
mod tests;
