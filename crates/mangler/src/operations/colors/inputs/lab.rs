use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputLab {}

impl OpColorInputLab {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lab".to_string(),
            description: "Creates a color using the LAB color space.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("lightness".to_string(), Value::Decimal(50.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("green - red".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-128.0, 127.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("blue - yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-128.0, 127.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let l_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let a_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let b_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(l) = l_converted.unwrap() else { unreachable!() };
        let Value::Decimal(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };        
        
        // run node
        let color = Color::from_lab(l, a, b, alpha);

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
            .map(|(i, v)| Input::new(format!("v{}",  i), Value::Decimal(*v), None, None))
            .collect()
    }

    #[tokio::test]
    async fn test_lab_input() {
        let mut inputs = decimal_inputs(&[50.0, 20.0, -30.0, 1.0]);
        let result = OpColorInputLab::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lab_settings() {
        let s = OpColorInputLab::settings();
        assert_eq!(s.name, "lab");
        assert_eq!(OpColorInputLab::create_inputs().len(), 4);
        assert_eq!(OpColorInputLab::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_lab_black() {
        // L=0, a=0, b=0 should give black
        let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 1.0]);
        let result = OpColorInputLab::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                assert!(r.abs() < 0.02, "black R should be ~0, got {}", r);
                assert!(g.abs() < 0.02, "black G should be ~0, got {}", g);
                assert!(b.abs() < 0.02, "black B should be ~0, got {}", b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lab_zero_alpha() {
        let mut inputs = decimal_inputs(&[50.0, 0.0, 0.0, 0.0]);
        let result = OpColorInputLab::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (_, _, _, a) = c.to_srgb_float();
                assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lab_produces_color() {
        // Various Lab values should produce a Color without panicking
        for (l, a, b) in [(0.0f32, 0.0f32, 0.0f32), (50.0, 25.0, -25.0), (100.0, 0.0, 0.0)] {
            let mut inputs = decimal_inputs(&[l, a, b, 1.0]);
            let result = OpColorInputLab::run(&mut inputs).await;
            assert!(result.is_ok(), "lab ({},{},{}) failed: {:?}", l, a, b, result.err());
        }
    }
}
