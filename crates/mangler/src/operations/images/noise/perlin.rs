use image::{ImageBuffer, DynamicImage};
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use noise::{NoiseFn, Perlin};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoisePerlin {}

impl OpImageNoisePerlin {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "perlin noise".to_string(),
            description: "Creates an image from perlin noise.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("scale".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.01, 100.0), step_by: Some(0.01), clamp_to_range:false }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let seed_converted = inputs[0].value.try_convert_to(ValueType::Integer);
        let width_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let height_converted = inputs[2].value.try_convert_to(ValueType::Integer);
        let scale_converted = inputs[3].value.try_convert_to(ValueType::Decimal);

        // gather errors
        if seed_converted.is_err() { input_errors.push((0, seed_converted.as_ref().err().unwrap().message.clone())); }
        if width_converted.is_err() { input_errors.push((1, width_converted.as_ref().err().unwrap().message.clone())); }
        if height_converted.is_err() { input_errors.push((2, height_converted.as_ref().err().unwrap().message.clone())); }
        if scale_converted.is_err() { input_errors.push((3, scale_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Integer(mut seed)) = seed_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut width)) = width_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut height)) = height_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(mut scale)) = scale_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        scale = scale.max(0.0001);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let perlin = Perlin::new(seed as u32);

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
