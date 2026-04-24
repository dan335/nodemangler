//! LCH color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from lightness, chroma, and hue
//! channel values. LCH is the cylindrical representation of CIE L*a*b*,
//! offering intuitive control over colorfulness and hue angle.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from LCH (Lightness, Chroma, Hue) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputLch {}

impl OpColorInputLch {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lch".to_string(),
            description: "Creates a color using the LCH color space.".to_string(),
            help: "Builds an sRGB color from the cylindrical form of Lab: lightness, chroma (radial colorfulness), and hue (0-360 angle). LCH inherits Lab's perceptual uniformity while giving intuitive polar control, so rotating hue at fixed chroma keeps the perceived vividness steady.\n\nHigh chroma values easily land outside the sRGB gamut and will be clipped at conversion time, so bright saturated hues can shift; reduce chroma for reliable results. Alpha is carried through without premultiplication.".to_string(),
        }
    }

    /// Creates the input definitions: lightness (0..2), chroma (0..1), hue (0..360), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("lightness".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Perceived lightness (0 dark to ~2 very bright) in LCH."),
            Input::new("chroma".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Colorfulness (0 = gray, higher = more vivid) in LCH."),
            Input::new("hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Hue angle in degrees (0–360) around the LCH color wheel."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the LCH + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from LCH float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let l_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let c_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let h_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(l) = l_converted.unwrap() else { unreachable!() };
        let Value::Decimal(c) = c_converted.unwrap() else { unreachable!() };
        let Value::Decimal(h) = h_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_lch(l, c, h, alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "lch_tests.rs"]
mod tests;
