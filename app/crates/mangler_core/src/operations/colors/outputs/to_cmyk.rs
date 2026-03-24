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
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: cyan, magenta, yellow, key (black), and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("cyan".to_string(), Value::Decimal(0.5), None),
            Output::new("magenta".to_string(), Value::Decimal(0.5), None),
            Output::new("yellow".to_string(), Value::Decimal(0.5), None),
            Output::new("key (black)".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
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

        Ok(OperationResponse { ai_cost_usd: None,
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
