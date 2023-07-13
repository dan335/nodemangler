use image::{RgbaImage, ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::color::color_spaces::rgb_linear::{nonlinear_to_linear_rgb, linear_to_nonlinear_srgb};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use noise::{NoiseFn, Perlin, Seedable, OpenSimplex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseValue {}

impl OpImageNoiseValue {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "value noise".to_string(),
            description: "Creates an image from value noise.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("scale".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.01, 100.0), step_by: Some(0.1), clamp_to_range:false }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Integer(mut seed)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut width)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(mut scale)) = inputs[3].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        scale = scale.max(0.0001);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let perlin = noise::Value::new(seed as u32);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64) * scale as f64;
                let coords_y = (y as f64) / (size as f64) * scale as f64;
                let noise = perlin.get([coords_x, coords_y]) as f32 * 0.5 + 0.5;
                let non_linear = linear_to_nonlinear_srgb(noise);
                let g = (non_linear * 255.0) as u8;
                image_buffer.put_pixel(x as u32, y as u32, image::Luma([g]));
            }
        }
        
        let dynamic_image = DynamicImage::ImageLuma8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: dynamic_image, change_id: get_id() } },
            ],
        })
    }
}
