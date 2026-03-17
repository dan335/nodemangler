use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlend {}

impl OpImageCombineBlend {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blend".to_string(),
            description: "Blits an image onto another image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("foreground".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("alpha".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(crate::color::blend::BlendMode::Normal), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let background_converted = inputs[0].value.try_convert_to(ValueType::DynamicImage);
        let foreground_converted = inputs[1].value.try_convert_to(ValueType::DynamicImage);
        let amount_converted = inputs[2].value.try_convert_to(ValueType::Decimal);
        let alpha_converted = inputs[3].value.try_convert_to(ValueType::DynamicImage);
        let blend_mode_converted = inputs[4].value.try_convert_to(ValueType::BlendMode);
        let color_space_converted = inputs[5].value.try_convert_to(ValueType::ColorSpace);
        let position_x_converted = inputs[6].value.try_convert_to(ValueType::Integer);
        let position_y_converted = inputs[7].value.try_convert_to(ValueType::Integer);

        // gather errors
        if background_converted.is_err() { input_errors.push((0, background_converted.as_ref().err().unwrap().message.clone())); }
        if foreground_converted.is_err() { input_errors.push((1, foreground_converted.as_ref().err().unwrap().message.clone())); }
        if amount_converted.is_err() { input_errors.push((2, amount_converted.as_ref().err().unwrap().message.clone())); }
        if alpha_converted.is_err() { input_errors.push((3, alpha_converted.as_ref().err().unwrap().message.clone())); }
        if blend_mode_converted.is_err() { input_errors.push((4, blend_mode_converted.as_ref().err().unwrap().message.clone())); }
        if color_space_converted.is_err() { input_errors.push((5, color_space_converted.as_ref().err().unwrap().message.clone())); }
        if position_x_converted.is_err() { input_errors.push((6, position_x_converted.as_ref().err().unwrap().message.clone())); }
        if position_y_converted.is_err() { input_errors.push((7, position_y_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::DynamicImage{data:background, change_id:_}) = background_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::DynamicImage{data:foreground, change_id:_}) = foreground_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(amount)) = amount_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::DynamicImage{data:alpha, change_id:_}) = alpha_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::BlendMode(blend_mode)) = blend_mode_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::ColorSpace(color_space)) = color_space_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut position_x)) = position_x_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut position_y)) = position_y_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        let mut background_image = background.to_rgba32f();
        let foreground_image = foreground.to_rgba32f();
        let alpha_image = alpha.to_rgb32f();
        position_x = position_x.max(0);
        position_y = position_y.max(0);

        for (x, y, pixel) in background_image.enumerate_pixels_mut() {
            let background_color = Color::from_srgb_float(pixel[0], pixel[1], pixel[2], pixel[3]);

            let foreground_x = x as i32 - position_x;
            let foreground_y = y as i32 - position_y;

            if foreground_x >= 0 && foreground_y >= 0 {
                if let Some(foreground_pixel) = foreground_image.get_pixel_checked(foreground_x as u32, foreground_y as u32) {
                    let mut blend_amount = amount;
    
                    if let Some(alpha_pixel) = alpha_image.get_pixel_checked(x, y) {
                        blend_amount = amount * ((alpha_pixel[0] as f32 + alpha_pixel[1] as f32 + alpha_pixel[2] as f32) / (1.0 * 3.0));
                    }
    
                    let foreground_color = Color::from_srgb_float(foreground_pixel[0], foreground_pixel[1], foreground_pixel[2], foreground_pixel[3]);
    
                    let new_color = match color_space {
                        crate::color::color_spaces::ColorSpace::Srgb => Color::blend_srgb(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::RgbLinear => Color::blend_linear(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Hsl => Color::blend_hsl(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Hsv => Color::blend_hsv(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Lch => Color::blend_lch(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Xyz => Color::blend_xyz(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Lab => Color::blend_lab(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Yuv => Color::blend_yuv(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Cmyk => Color::blend_cmyk(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                    };
    
                    *pixel = image::Rgba([new_color.0, new_color.1, new_color.2, new_color.3]);
                }
            }

            
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba32F(background_image)), change_id:get_id() }},
            ],
        })
    }
}
