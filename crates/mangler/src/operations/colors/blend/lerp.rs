use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorBlendLerp {}

impl OpColorBlendLerp {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blend".to_string(),
            description: "Blends two colors together by lerping between them in a specific color space.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None),
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
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let amount_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let color_space_converted = convert_input(inputs, 3, ValueType::ColorSpace, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::ColorSpace(color_space) = color_space_converted.unwrap() else { unreachable!() };  

        // run node
        let color = match color_space {
            crate::color::color_spaces::ColorSpace::Srgb => Color::blend_srgb(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::RgbLinear => Color::blend_linear(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Hsl => Color::blend_hsl(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Hsv => Color::blend_hsv(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Lch => Color::blend_lch(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Xyz => Color::blend_xyz(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Lab => Color::blend_lab(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Yuv => Color::blend_yuv(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Cmyk => Color::blend_cmyk(a, b, &crate::color::blend::BlendMode::Lerp, amount),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}
