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
pub struct OpImageTransformResize {}

impl OpImageTransformResize {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resize".to_string(),
            description: "Resizes an image to a specific size.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("width".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
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

        let Ok(Value::Integer(mut width)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::FilterType(filter_type)) = inputs[3].value.try_convert_to(ValueType::FilterType) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        width = width.max(1);
        height = height.max(1);

        let Value::DynamicImage{data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        let resized = data.resize(width as u32, height as u32, filter_type);

        // let resized = tokio::spawn(async move {
        //     let resized = data.resize(width as u32, height as u32, filter_type);
        //     resized
        // }).await.unwrap();

        let value_width = Value::Integer(resized.width() as i32);
        let value_height = Value::Integer(resized.height() as i32);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:resized, change_id:get_id() }},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}
