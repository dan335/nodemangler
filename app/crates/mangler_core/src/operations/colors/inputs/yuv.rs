//! YUV color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from Y (luminance), U (blue
//! chrominance), V (red chrominance), and alpha channel values. YUV separates
//! brightness from color information, as used in video encoding standards.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from YUV (luminance + chrominance) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputYuv {}

impl OpColorInputYuv {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "yuv".to_string(),
            description: "Creates a color using the YUV color space.".to_string(),
            help: "Builds an sRGB color from Y (luminance, 0-1), U (blue-difference chroma), and V (red-difference chroma). YUV separates brightness from color the way analog TV and modern video codecs do, so tweaking Y alone changes overall brightness while leaving hue roughly stable.\n\nU and V are expected as the standard video-range offsets (centered around 0.5 gives a neutral gray). Saturated or out-of-gamut YUV triplets will clip to the sRGB 0-1 range after conversion. Alpha is passed through straight.".to_string(),
        }
    }

    /// Creates the input definitions: Y (luminance), U (blue chrominance), V (red chrominance), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("y (luminance)".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Y luminance channel (0 = black, 1 = white) of the YUV color."),
            Input::new("u (chrominance blue)".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("U blue-difference chrominance of the YUV color."),
            Input::new("v (chrominance red)".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("V red-difference chrominance of the YUV color."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the YUV + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from YUV float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let y_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let u_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let v_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(u) = u_converted.unwrap() else { unreachable!() };
        let Value::Decimal(v) = v_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_yuv(y, u, v, alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "yuv_tests.rs"]
mod tests;
