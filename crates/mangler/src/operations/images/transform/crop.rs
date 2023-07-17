use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformCrop {}

impl OpImageTransformCrop {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "crop".to_string(),
            description: "Crops an image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("x".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("y".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = inputs[0].value.try_convert_to(ValueType::DynamicImage);
        let x_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let y_converted = inputs[2].value.try_convert_to(ValueType::Integer);
        let width_converted = inputs[3].value.try_convert_to(ValueType::Integer);
        let height_converted = inputs[4].value.try_convert_to(ValueType::Integer);

        // gather errors
        if image_converted.is_err() { input_errors.push((0, image_converted.as_ref().err().unwrap().message.clone())); }
        if x_converted.is_err() { input_errors.push((1, x_converted.as_ref().err().unwrap().message.clone())); }
        if y_converted.is_err() { input_errors.push((2, y_converted.as_ref().err().unwrap().message.clone())); }
        if width_converted.is_err() { input_errors.push((3, width_converted.as_ref().err().unwrap().message.clone())); }
        if height_converted.is_err() { input_errors.push((4, height_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::DynamicImage{mut data, change_id:_}) = image_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut x)) = x_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut y)) = y_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut width)) = width_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut height)) = height_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        x = x.max(0).min(data.width() as i32 - 1);
        y = y.max(0).min(data.height() as i32 - 1);
        width = width.max(1).min(data.width() as i32);
        height = height.max(1).min(data.height() as i32);

        let resized = image::imageops::crop(&mut data, x as u32, y as u32, width as u32, height as u32).to_image();

        let value_width = Value::Integer(resized.width() as i32);
        let value_height = Value::Integer(resized.height() as i32);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:image::DynamicImage::ImageRgba8(resized), change_id:get_id() }},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}
