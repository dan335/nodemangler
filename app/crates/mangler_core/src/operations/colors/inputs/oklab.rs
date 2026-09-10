//! Oklab color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from Oklab lightness (L, 0..1),
//! green-red axis (a), blue-yellow axis (b), and alpha.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from Oklab channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputOklab {}

impl OpColorInputOklab {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "oklab".to_string(),
            description: "Creates a color using the Oklab color space.".to_string(),
            help: "Builds an sRGB color from Oklab (Björn Ottosson, 2020): L is perceptual lightness (0 black to 1 white), a is the green-red axis, and b is the blue-yellow axis (each roughly -0.4..0.4).\n\nOklab is perceptually uniform and designed for predictable mixing, so it is well suited to gradients and lightness adjustments. Values far from the achromatic axis can fall outside the sRGB gamut and will be clipped. Alpha is passed through unchanged.".to_string(),
        }
    }

    /// Creates the input definitions: L (0..1), a, b (≈ -0.4..0.4), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("lightness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Oklab L: perceptual lightness (0 = black, 1 = white)."),
            Input::new("green - red".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.4, 0.4), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Oklab a axis: negative toward green, positive toward red."),
            Input::new("blue - yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.4, 0.4), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Oklab b axis: negative toward blue, positive toward yellow."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the Oklab + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from Oklab float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Decimal(l) = 0,
            Decimal(a) = 1,
            Decimal(b) = 2,
            Decimal(alpha) = 3,
        }

        let color = Color::from_oklab(l, a, b, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Color(color) }],
        })
    }
}

#[cfg(test)]
#[path = "oklab_tests.rs"]
mod tests;
