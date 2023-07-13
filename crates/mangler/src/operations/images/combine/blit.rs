use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlit {}

impl OpImageCombineBlit {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blit".to_string(),
            description: "Blits an image onto another image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("foreground".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Value::DynamicImage{data:mut image_a, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };
        let Value::DynamicImage{data:image_b, change_id:_} = inputs[1].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        let Ok(Value::Integer(x)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(y)) = inputs[3].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        image::imageops::overlay(&mut image_a, &image_b, x as i64, y as i64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:image_a, change_id:get_id() }},
            ],
        })
    }
}
