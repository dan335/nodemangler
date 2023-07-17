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
use noise::{NoiseFn, Cylinders};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCylinders {}

impl OpImageNoiseCylinders {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cylinders noise".to_string(),
            description: "This noise function outputs concentric cylinders centered on the origin. The cylinders are oriented along the z axis similar to the concentric rings of a tree. Each cylinder extends infinitely along the z axis.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("frequency".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
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
        let width_converted = inputs[0].value.try_convert_to(ValueType::Integer);
        let height_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let frequency_converted = inputs[2].value.try_convert_to(ValueType::Decimal);

        // gather errors
        if width_converted.is_err() { input_errors.push((0, width_converted.as_ref().err().unwrap().message.clone())); }
        if height_converted.is_err() { input_errors.push((1, height_converted.as_ref().err().unwrap().message.clone())); }
        if frequency_converted.is_err() { input_errors.push((2, frequency_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Integer(mut width)) = width_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut height)) = height_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(frequency)) = frequency_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        width = width.max(1);
        height = height.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let perlin = Cylinders::new().set_frequency(frequency as f64);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64);
                let coords_y = (y as f64) / (size as f64);
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
