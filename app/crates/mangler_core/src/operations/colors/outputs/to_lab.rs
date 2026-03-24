//! CIE L*a*b* color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into lightness (L*),
//! green-red axis (a*), blue-yellow axis (b*), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CIE L*a*b* channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputLab {}

impl OpColorOutputLab {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to lab".to_string(),
            description: "Converts a color to the LAB color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: lightness, green-red (a*), blue-yellow (b*), and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("lightness".to_string(), Value::Decimal(0.5), None),
            Output::new("green - red".to_string(), Value::Decimal(0.5), None),
            Output::new("blue - yellow".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to CIE L*a*b* float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (l, a, b, alpha) = color.to_lab();

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(a)},
                OutputResponse {value: Value::Decimal(b)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_lab_tests.rs"]
mod tests;
