use image::{RgbaImage, ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputGradient {}

impl OpImageInputGradient {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from gradient".to_string(),
            description: "Creates an image from a gradient.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_u8(255, 255, 255, 255)), None, None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("color".to_string(), Value::Color(Color::default()), None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Color(a)) = inputs[0].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Color(b)) = inputs[1].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let Ok(Value::Integer(mut width)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[3].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        
        let Ok(Value::ColorSpace(color_space)) = inputs[4].value.try_convert_to(ValueType::ColorSpace) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        width = width.max(1);
        height = height.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        match color_space {
            ColorSpace::Srgb => {
                for y in 0..height {
                    let blended = Color::blend_srgb(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::RgbLinear => {
                for y in 0..height {
                    let blended = Color::blend_linear(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Hsl => {
                for y in 0..height {
                    let blended = Color::blend_hsl(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Hsv => {
                for y in 0..height {
                    let blended = Color::blend_hsv(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Lch => {
                for y in 0..height {
                    let blended = Color::blend_lch(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Xyz => {
                for y in 0..height {
                    let blended = Color::blend_xyz(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Lab => {
                for y in 0..height {
                    let blended = Color::blend_lab(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Yuv => {
                for y in 0..height {
                    let blended = Color::blend_yuv(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Cmyk => {
                for y in 0..height {
                    let blended = Color::blend_cmyk(a, b, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
        }

        
        
        let dynamic_image = DynamicImage::ImageRgba8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: dynamic_image, change_id: get_id() } },
                OutputResponse { value: Value::Integer(width as i32) },
                OutputResponse { value: Value::Integer(height as i32) },
            ],
        })
    }
}
