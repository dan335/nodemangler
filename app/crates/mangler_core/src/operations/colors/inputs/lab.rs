//! CIE L*a*b* color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from lightness (L*, 0..100),
//! green-red axis (a*, -128..127), blue-yellow axis (b*, -128..127), and alpha.
//! L*a*b* is a perceptually uniform color space.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from CIE L*a*b* channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputLab {}

impl OpColorInputLab {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lab".to_string(),
            description: "Creates a color using the LAB color space.".to_string(),
        }
    }

    /// Creates the input definitions: L* (0..100), a* (-128..127), b* (-128..127), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("lightness".to_string(), Value::Decimal(50.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("green - red".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-128.0, 127.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("blue - yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-128.0, 127.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    /// Executes the operation, assembling a color from CIE L*a*b* float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let l_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let a_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let b_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(l) = l_converted.unwrap() else { unreachable!() };
        let Value::Decimal(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };        
        
        // run node
        let color = Color::from_lab(l, a, b, alpha);

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "lab_tests.rs"]
mod tests;
