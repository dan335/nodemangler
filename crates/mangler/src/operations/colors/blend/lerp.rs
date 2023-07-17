use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
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
        let a_converted = inputs[0].value.try_convert_to(ValueType::Color);
        let b_converted = inputs[1].value.try_convert_to(ValueType::Color);
        let amount_converted = inputs[2].value.try_convert_to(ValueType::Decimal);
        let color_space_converted = inputs[3].value.try_convert_to(ValueType::ColorSpace);

        // gather errors
        if a_converted.is_err() { input_errors.push((0, a_converted.as_ref().err().unwrap().message.clone())); }
        if b_converted.is_err() { input_errors.push((0, b_converted.as_ref().err().unwrap().message.clone())); }
        if amount_converted.is_err() { input_errors.push((0, amount_converted.as_ref().err().unwrap().message.clone())); }
        if color_space_converted.is_err() { input_errors.push((0, color_space_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Color(a)) = a_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Color(b)) = b_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(amount)) = amount_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::ColorSpace(color_space)) = color_space_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };  

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
