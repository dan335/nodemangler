use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputYuv {}

impl OpColorInputYuv {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "yuv".to_string(),
            description: "Creates a color using the YUV color space.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("y (luminance)".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("u (chrominance blue)".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("v (chrominance red)".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
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
        let y_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let u_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let v_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(u) = u_converted.unwrap() else { unreachable!() };
        let Value::Decimal(v) = v_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_yuv(y, u, v, alpha);

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
    async fn test_yuv_input() {
        let mut inputs = decimal_inputs(&[0.5, 0.3, 0.2, 1.0]);
        let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_yuv_settings() {
        let s = OpColorInputYuv::settings();
        assert_eq!(s.name, "yuv");
        assert_eq!(OpColorInputYuv::create_inputs().len(), 4);
        assert_eq!(OpColorInputYuv::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_yuv_zero_alpha() {
        let mut inputs = decimal_inputs(&[0.5, 0.3, 0.2, 0.0]);
        let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (_, _, _, a) = c.to_srgb_float();
                assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_yuv_produces_color() {
        // Various YUV values should produce a Color without panicking
        for (y, u, v) in [(0.0f32, 0.0f32, 0.0f32), (0.5, 0.0, 0.0), (1.0, 0.5, 0.5)] {
            let mut inputs = decimal_inputs(&[y, u, v, 1.0]);
            let result = OpColorInputYuv::run(&mut inputs).await;
            assert!(result.is_ok(), "yuv ({},{},{}) failed: {:?}", y, u, v, result.err());
        }
    }

    #[tokio::test]
    async fn test_yuv_neutral_chrominance() {
        // Y=0.5, U=0, V=0 should produce a neutral grey
        let mut inputs = decimal_inputs(&[0.5, 0.0, 0.0, 1.0]);
        let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, g, b, _) = c.to_srgb_float();
                // Neutral grey: R≈G≈B
                assert!((r - g).abs() < 0.05, "neutral grey R≈G failed: r={}, g={}", r, g);
                assert!((g - b).abs() < 0.05, "neutral grey G≈B failed: g={}, b={}", g, b);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }
}
