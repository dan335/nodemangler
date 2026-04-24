//! LCH color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into lightness, chroma, hue,
//! and alpha channel values. LCH is the cylindrical form of CIE L*a*b*.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into LCH (Lightness, Chroma, Hue) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputLch {}

impl OpColorOutputLch {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to lch".to_string(),
            description: "Converts a color to the LCH color space.".to_string(),
            help: "Converts the color to Lab and then expresses the a*/b* pair as polar (chroma, hue) while keeping L* as lightness. Chroma is the distance from the neutral axis (colorfulness) and hue is the angle in degrees (0-360).\n\nFor near-neutral grays where chroma is essentially zero, hue is not meaningful and will be reported as 0 rather than NaN. Use this when you want perceptually uniform lightness with intuitive hue rotation. Alpha is passed through from the input.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into LCH channels."),
        ]
    }

    /// Creates the output definitions: lightness, chroma, hue, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("lightness".to_string(), Value::Decimal(0.5), None)
                .with_description("LCH perceived lightness of the input color."),
            Output::new("chroma".to_string(), Value::Decimal(0.5), None)
                .with_description("LCH chroma (colorfulness) of the input color."),
            Output::new("hue".to_string(), Value::Decimal(0.5), None)
                .with_description("LCH hue angle in degrees (0–360)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to LCH float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (l, c, h, a) = color.to_lch();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(c)},
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_lch_tests.rs"]
mod tests;
