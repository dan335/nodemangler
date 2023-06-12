use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageAdjustmentGrayscale {}

impl OperationImageAdjustmentGrayscale {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grayscale".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id() }, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Value::DynamicImage{data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        let adjusted = data.grayscale();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:adjusted, change_id:get_id() }},
            ],
        })
    }
}
