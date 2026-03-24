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
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: Y (luminance), U (chrominance blue), V (chrominance red), and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("y (luminance)".to_string(), Value::Decimal(0.5), None),
            Output::new("u (chrominance blue)".to_string(), Value::Decimal(0.5), None),
            Output::new("v (chrominance red)".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
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

        Ok(OperationResponse { ai_cost_usd: None,
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
