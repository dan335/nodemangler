//! HSV color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into hue, saturation, value
//! (brightness), and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into HSV (Hue, Saturation, Value) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputHsv {}

impl OpColorOutputHsv {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to hsv".to_string(),
            description: "Converts a color to the HSV color space.".to_string(),
            help: "Splits the color into hue (0-360 degrees), saturation (0-1), value/brightness (0-1), and alpha. In HSV the pure saturated hue lives at value 1, contrasting with HSL where it sits at lightness 0.5.\n\nFor pure grays (max(R, G, B) == min(R, G, B)) hue is reported as 0 and saturation as 0 to avoid NaN. Alpha is passed through from the input untouched.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into HSV channels."),
        ]
    }

    /// Creates the output definitions: hue, saturation, value, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hue".to_string(), Value::Decimal(0.5), None)
                .with_description("Hue angle in degrees (0–360) extracted from the input color."),
            Output::new("saturation".to_string(), Value::Decimal(0.5), None)
                .with_description("HSV saturation (0 = gray, 1 = fully saturated)."),
            Output::new("value".to_string(), Value::Decimal(0.5), None)
                .with_description("HSV value/brightness (0 = black, 1 = full brightness)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to HSV float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (h, s, v, a) = color.to_hsv();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(s)},
                OutputResponse {value: Value::Decimal(v)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_hsv_tests.rs"]
mod tests;
