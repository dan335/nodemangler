//! YCbCr (BT.709) color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into luma (Y), blue-difference
//! chroma (Cb), red-difference chroma (Cr), and alpha, using Rec. 709 (HD).

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into BT.709 YCbCr channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputYcbcr {}

impl OpColorOutputYcbcr {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to ycbcr".to_string(),
            description: "Converts a color to the YCbCr (BT.709) color space.".to_string(),
            help: "Splits the color into digital YCbCr using Rec. 709 (HD) coefficients, full range: Y is luma (0..1), Cb is blue-difference chroma and Cr is red-difference chroma (each -0.5..0.5, 0 = neutral). Alpha is forwarded unchanged.\n\nThis is the luma/chroma encoding used by H.264/H.265 HD video, distinct from the analog BT.601 Y'UV node.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into YCbCr channels."),
        ]
    }

    /// Creates the output definitions: luma, Cb, Cr, and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("luma".to_string(), Value::Decimal(0.0), None)
                .with_description("Y: luma / brightness (0..1)."),
            Output::new("cb".to_string(), Value::Decimal(0.0), None)
                .with_description("Cb: blue-difference chroma (0 = neutral)."),
            Output::new("cr".to_string(), Value::Decimal(0.0), None)
                .with_description("Cr: red-difference chroma (0 = neutral)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to BT.709 YCbCr float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (y, cb, cr, alpha) = color.to_ycbcr();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(cb)},
                OutputResponse {value: Value::Decimal(cr)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_ycbcr_tests.rs"]
mod tests;
