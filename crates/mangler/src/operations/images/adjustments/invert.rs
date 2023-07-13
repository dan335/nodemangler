use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentInvert {}

impl OpImageAdjustmentInvert {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "invert".to_string(),
            description: "Inverts the colors of an image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let Value::DynamicImage{mut data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        data.invert();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data, change_id:get_id() }},
            ],
        })
    }
}
