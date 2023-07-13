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

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Value::DynamicImage{mut data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };
        let Ok(Value::Integer(mut x)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut y)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut width)) = inputs[3].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[4].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
    
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
