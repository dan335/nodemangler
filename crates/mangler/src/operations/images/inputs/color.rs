use image::{RgbaImage, ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputColor {}

impl OpImageInputColor {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from color".to_string(),
            description: "Creates an image from a color.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new("width".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
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

        let Ok(Value::Color(color)) = inputs[0].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let Ok(Value::Integer(mut width)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        
        width = width.max(1);
        height = height.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);
        let rgba = color.to_srgb_u8();

        for x in 0..width {
            for y in 0..height {
                image_buffer.put_pixel(x as u32, y as u32, image::Rgba([rgba.0, rgba.1, rgba.2, rgba.3]));
            }
        }
        
        let dynamic_image = DynamicImage::ImageRgba8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: dynamic_image, change_id: get_id() } },
                OutputResponse { value: Value::Color(color) },
                OutputResponse { value: Value::Integer(width as i32) },
                OutputResponse { value: Value::Integer(height as i32) },
            ],
        })
    }
}
