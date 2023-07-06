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
pub struct OpImageAdjustmentContrast {}

impl OpImageAdjustmentContrast {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "contrast".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Decimal(amount)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let Value::DynamicImage{data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        let adjusted = data.adjust_contrast(amount);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:adjusted, change_id:get_id() }},
            ],
        })
    }
}
