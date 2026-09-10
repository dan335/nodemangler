//! Color clamp operation.
//!
//! Clamps the RGB channels of a color to a user-specified [min, max] range.
//! The alpha channel is passed through unchanged.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that clamps the RGB channels of a color to a specified min/max range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorManipulationClamp {}

impl OpColorManipulationClamp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clamp".to_string(),
            description: "Clamps the RGB channels of a color to a specified min/max range.".to_string(),
            help: "Constrains each of the R, G, and B channels independently to the [min, max] window using a per-channel clamp. Useful for forcing a color into a safe range (for example keeping channels away from pure 0 or 1 to avoid clipping in downstream effects).\n\nThe operation works directly on the stored sRGB floats, not in linear light, so it is fine for display-referred compositing but not for physically correct exposure limits. Alpha is passed through unchanged, and if min > max every channel collapses to min.".to_string(),
        }
    }

    /// Creates the input definitions: a color and min/max sliders.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color whose RGB channels will be clamped."),
            Input::new(
                "min".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            )
            .with_description("Lower bound each RGB channel is clamped to."),
            Input::new(
                "max".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            )
            .with_description("Upper bound each RGB channel is clamped to."),
        ]
    }

    /// Creates the single output definition for the clamped color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color with each RGB channel clamped to [min, max]; alpha is preserved."),
        ]
    }

    /// Executes the clamp operation, constraining each RGB channel to [min, max].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Color(color) = 0,
            Decimal(min) = 1,
            Decimal(max) = 2,
        }

        // Clamp each RGB channel; alpha is preserved as-is
        let result = Color::from_srgb_float(
            color.r.clamp(min, max),
            color.g.clamp(min, max),
            color.b.clamp(min, max),
            color.a,
        );

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "clamp_tests.rs"]
mod tests;
