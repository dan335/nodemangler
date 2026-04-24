//! YUV color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into Y (luminance),
//! U (blue chrominance), V (red chrominance), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into YUV (luminance + chrominance) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputYuv {}

impl OpColorOutputYuv {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to yuv".to_string(),
            description: "Converts a color to the YUV color space.".to_string(),
            help: "Splits the color into Y (luminance/brightness) and the two chrominance channels U (blue-difference) and V (red-difference). This matches the family of transforms used by broadcast video and many codecs, where brightness and color can be processed independently.\n\nY is output in 0-1 and U/V are centered so that a gray input produces U ~= V ~= 0.5. Alpha passes through unchanged. Use this when you want to tweak luma or chroma separately, or to match a video-style workflow.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into YUV luminance and chrominance channels."),
        ]
    }

    /// Creates the output definitions: Y (luminance), U (chrominance blue), V (chrominance red), and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("y (luminance)".to_string(), Value::Decimal(0.5), None)
                .with_description("Y luminance (brightness) channel of the input color."),
            Output::new("u (chrominance blue)".to_string(), Value::Decimal(0.5), None)
                .with_description("U blue-difference chrominance channel of the input color."),
            Output::new("v (chrominance red)".to_string(), Value::Decimal(0.5), None)
                .with_description("V red-difference chrominance channel of the input color."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to YUV float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (y, u, v, alpha) = color.to_yuv();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(u)},
                OutputResponse {value: Value::Decimal(v)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_yuv_tests.rs"]
mod tests;
