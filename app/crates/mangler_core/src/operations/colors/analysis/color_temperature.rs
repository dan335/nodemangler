//! Color temperature estimation operation.
//!
//! Estimates the perceptual color temperature of a color in Kelvin using the
//! McCamy formula applied to CIE XYZ chromaticity coordinates. Also outputs
//! a normalized warm-cool value (0 = cool/blue, 1 = warm/red-orange).

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that estimates the perceptual color temperature using the McCamy formula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisColorTemperature {}

impl OpColorAnalysisColorTemperature {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "color temperature".to_string(),
            description: "Estimates the perceptual color temperature in Kelvin using the McCamy formula via XYZ chromaticity. Also outputs a normalized warm (1.0) to cool (0.0) value.".to_string(),
            help: "Converts the input to CIE XYZ, computes chromaticity coordinates (cx, cy), and evaluates McCamy's cubic polynomial to estimate correlated color temperature (CCT).\n\nThe result is clamped to the physically meaningful 1000 K (candle) to 20000 K (blue sky) range, with 6504 K used as a D65 fallback when the color is too dark for chromaticity to be defined. warm_cool remaps this so 1.0 is warm red-orange and 0.0 is cool blue.\n\nNote that CCT is only strictly meaningful for near-neutral whites; saturated colors far off the Planckian locus still return a value but treat it as a rough perceptual warmth cue.".to_string(),
        }
    }

    /// Creates the single input definition: the source color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to estimate the correlated color temperature of."),
        ]
    }

    /// Creates the two output definitions: kelvin and warm_cool.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("kelvin".to_string(), Value::Decimal(0.0), None)
                .with_description("Estimated correlated color temperature in Kelvin (1000–20000)."),
            Output::new("warm_cool".to_string(), Value::Decimal(0.0), None)
                .with_description("Normalized 0 (cool/blue) to 1 (warm/red-orange) temperature position."),
        ]
    }

    /// Executes the color temperature estimation.
    ///
    /// Steps:
    /// 1. Convert to XYZ and compute CIE chromaticity (cx, cy).
    /// 2. Apply McCamy's formula to get correlated color temperature in Kelvin.
    /// 3. Clamp to 1000–20000 K and normalize to a 0 (cool) – 1 (warm) value.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert input color.
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted value.
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // Convert to XYZ (already normalized: white has X+Y+Z ≈ 1).
        let (x, y, z, _alpha) = color.to_xyz();

        // Compute the sum to get CIE chromaticity coordinates.
        let sum = x + y + z;

        // Guard against very dark colors where chromaticity is undefined; fall back to D65 neutral.
        let kelvin = if sum < 1e-6 {
            6504.0_f32
        } else {
            // CIE chromaticity: cx and cy are the fractional contributions of X and Y.
            let cx = x / sum;
            let cy = y / sum;

            // McCamy's approximation for correlated color temperature.
            // n is the epicenter distance from the neutral point.
            let n = (cx - 0.3320) / (0.1858 - cy);

            // Polynomial expansion of McCamy's formula.
            let k = 449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5520.33;

            // Clamp to the physically meaningful range (1000K candle → 20000K blue sky).
            k.clamp(1000.0, 20000.0)
        };

        // Normalize to a 0.0 (cool, ~20000K) to 1.0 (warm, ~1000K) scale.
        let warm_cool = (1.0 - (kelvin - 1000.0) / 19000.0).clamp(0.0, 1.0);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(kelvin) },
                OutputResponse { value: Value::Decimal(warm_cool) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "color_temperature_tests.rs"]
mod tests;
