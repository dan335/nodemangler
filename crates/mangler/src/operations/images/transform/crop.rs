use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut x) = x_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut y) = y_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };

        // run node
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        x = x.max(0).min(data_inner.width() as i32 - 1);
        y = y.max(0).min(data_inner.height() as i32 - 1);
        width = width.max(1).min(data_inner.width() as i32);
        height = height.max(1).min(data_inner.height() as i32);

        let resized = image::imageops::crop(&mut data_inner, x as u32, y as u32, width as u32, height as u32).to_image();

        let value_width = Value::Integer(resized.width() as i32);
        let value_height = Value::Integer(resized.height() as i32);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(image::DynamicImage::ImageRgba8(resized)), change_id:get_id() }},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}
