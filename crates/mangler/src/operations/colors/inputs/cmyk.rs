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
        }
    }

    /// Creates the input definitions: cyan, magenta, yellow, key (0..1 each), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("cyan".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("magenta".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("key (black)".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
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
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
        vals.iter()
            .enumerate()
            .map(|(i, v)| Input::new(format!("v{}", i), Value::Decimal(*v), None, None))
            .collect()
    }

    #[tokio::test]
    async fn test_cmyk_input() {
        let mut inputs = decimal_inputs(&[0.0, 1.0, 1.0, 0.0, 1.0]);
        let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cmyk_settings() {
        let s = OpColorInputCmyk::settings();
        assert_eq!(s.name, "cmyk");
        assert_eq!(OpColorInputCmyk::create_inputs().len(), 5);
        assert_eq!(OpColorInputCmyk::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_cmyk_black() {
        // K=1 (full black key) should give black
        let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 1.0, 1.0]);
        let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!(r.abs() < 0.02, "full-black R should be ~0, got {}", r);
                assert!(g.abs() < 0.02, "full-black G should be ~0, got {}", g);
                assert!(b.abs() < 0.02, "full-black B should be ~0, got {}", b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cmyk_white() {
        // All channels 0 should give white
        let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 0.0, 1.0]);
        let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!((r - 1.0).abs() < 0.02, "white R should be ~1, got {}", r);
                assert!((g - 1.0).abs() < 0.02, "white G should be ~1, got {}", g);
                assert!((b - 1.0).abs() < 0.02, "white B should be ~1, got {}", b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cmyk_zero_alpha() {
        let mut inputs = decimal_inputs(&[0.5, 0.5, 0.5, 0.5, 0.0]);
        let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (_, _, _, a) = c.to_srgb_float();
                assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }
}
