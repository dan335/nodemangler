//! CMYK color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from cyan, magenta, yellow,
//! key (black), and alpha channel values. CMYK is a subtractive color model
//! commonly used in print production.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from CMYK (Cyan, Magenta, Yellow, Key) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputCmyk {}

impl OpColorInputCmyk {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cmyk".to_string(),
            description: "Creates a color using the CMYK color space.".to_string(),
            help: "Assembles an sRGB color from the subtractive CMYK channels plus alpha. Cyan, magenta, yellow, and key each take 0-1 ink amounts; internally the conversion computes R = (1 - C)(1 - K), G = (1 - M)(1 - K), B = (1 - Y)(1 - K).\n\nThis is a math model of CMYK rather than an ICC-managed print conversion, so colors will not precisely match any specific press or paper profile. Alpha is carried through untouched and not premultiplied. Useful for print-style mixing or for driving graph inputs in a CMY/K-centric workflow.".to_string(),
        }
    }

    /// Creates the input definitions: cyan, magenta, yellow, key (0..1 each), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("cyan".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Cyan ink amount (0–1) in the CMYK subtractive model."),
            Input::new("magenta".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Magenta ink amount (0–1) in the CMYK subtractive model."),
            Input::new("yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Yellow ink amount (0–1) in the CMYK subtractive model."),
            Input::new("key (black)".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Key (black) ink amount (0–1); higher values darken the output."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the CMYK + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from CMYK float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let c_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let m_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let k_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(c) = c_converted.unwrap() else { unreachable!() };
        let Value::Decimal(m) = m_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(k) = k_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_cmyk(c, m, y, k, alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "cmyk_tests.rs"]
mod tests;
