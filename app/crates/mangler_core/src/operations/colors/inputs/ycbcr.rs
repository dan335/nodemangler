//! YCbCr (BT.709) color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from luma (Y), blue-difference
//! chroma (Cb), red-difference chroma (Cr), and alpha, using Rec. 709 (HD).

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from BT.709 YCbCr channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputYcbcr {}

impl OpColorInputYcbcr {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ycbcr".to_string(),
            description: "Creates a color using the YCbCr (BT.709) color space.".to_string(),
            help: "Builds an sRGB color from digital YCbCr using Rec. 709 (HD) coefficients, full range: Y is luma (brightness, 0..1), Cb is the blue-difference chroma and Cr the red-difference chroma (each -0.5..0.5, with 0 being neutral).\n\nThis is the luma/chroma encoding used by H.264/H.265 HD video, distinct from the analog BT.601 Y'UV node. Alpha is passed through unchanged.".to_string(),
        }
    }

    /// Creates the input definitions: Y (0..1), Cb, Cr (-0.5..0.5), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("luma".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Y: luma / brightness (0 = black, 1 = white)."),
            Input::new("cb".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Cb: blue-difference chroma (0 = neutral)."),
            Input::new("cr".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Cr: red-difference chroma (0 = neutral)."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the YCbCr + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from BT.709 YCbCr float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let y_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cb_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let cr_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cb) = cb_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cr) = cr_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        let color = Color::from_ycbcr(y, cb, cr, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Color(color) }],
        })
    }
}

#[cfg(test)]
#[path = "ycbcr_tests.rs"]
mod tests;
