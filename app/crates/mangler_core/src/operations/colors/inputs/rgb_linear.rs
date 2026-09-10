//! Linear RGB color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from red, green, blue, and alpha
//! channel values in the linear (non-gamma-encoded) RGB color space.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from linear RGB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputRgbaLinear {}

impl OpColorInputRgbaLinear {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rgb linear".to_string(),
            description: "Creates a color using the linear RGB color space.".to_string(),
            help: "Treats the r, g, and b sliders as values in a gamma-removed linear RGB space (the same primaries as sRGB but without the transfer curve) and applies the sRGB OETF internally before storing as a Color.\n\nUse this when your numbers come from physically linear sources like renderers or shader outputs, or when you want 0.5 to mean half as much light rather than mid-brightness. Visually 0.5 linear is noticeably lighter than 0.5 sRGB. Alpha passes through straight.".to_string(),
        }
    }

    /// Creates the input definitions: red, green, blue, and alpha sliders (0..1 range).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("red".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Red channel intensity (0–1) in linear, non-gamma-encoded RGB."),
            Input::new("green".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Green channel intensity (0–1) in linear, non-gamma-encoded RGB."),
            Input::new("blue".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Blue channel intensity (0–1) in linear, non-gamma-encoded RGB."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the linear RGB + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from linear RGB float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Decimal(red) = 0,
            Decimal(green) = 1,
            Decimal(blue) = 2,
            Decimal(alpha) = 3,
        }

        // run node
        let color = Color::from_rgb_linear(red, green, blue, alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "rgb_linear_tests.rs"]
mod tests;
