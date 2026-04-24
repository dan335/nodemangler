//! CMYK color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into cyan, magenta, yellow,
//! key (black), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CMYK (Cyan, Magenta, Yellow, Key) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputCmyk {}

impl OpColorOutputCmyk {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to cmyk".to_string(),
            description: "Converts a color to the CMYK color space.".to_string(),
            help: "Splits the input color into cyan, magenta, yellow, key (black), and alpha floats in 0-1. Internally K is computed as 1 - max(R, G, B), and C/M/Y are the remaining differences normalized by (1 - K); pure black collapses to K = 1 with C = M = Y = 0.\n\nThis is a generic CMYK model, not a color-managed print separation, so exact printer output will vary. Alpha is passed through untouched from the input.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into CMYK ink channels."),
        ]
    }

    /// Creates the output definitions: cyan, magenta, yellow, key (black), and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("cyan".to_string(), Value::Decimal(0.5), None)
                .with_description("Cyan ink amount (0–1) extracted from the input color."),
            Output::new("magenta".to_string(), Value::Decimal(0.5), None)
                .with_description("Magenta ink amount (0–1) extracted from the input color."),
            Output::new("yellow".to_string(), Value::Decimal(0.5), None)
                .with_description("Yellow ink amount (0–1) extracted from the input color."),
            Output::new("key (black)".to_string(), Value::Decimal(0.5), None)
                .with_description("Key (black) ink amount (0–1) extracted from the input color."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to CMYK float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        // run node
        let (c, m, y, k, a) = color.to_cmyk();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(c)},
                OutputResponse {value: Value::Decimal(m)},
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(k)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_cmyk_tests.rs"]
mod tests;
