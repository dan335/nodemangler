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
pub struct OpImageAdjustmentHueRotate{}

impl OpImageAdjustmentHueRotate {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hue rotate".to_string(),
            description: "Rotates the hue of an image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = inputs[0].value.try_convert_to(ValueType::DynamicImage);
        let amount_converted = inputs[1].value.try_convert_to(ValueType::Decimal);

        // gather errors
        if image_converted.is_err() { input_errors.push((0, image_converted.as_ref().err().unwrap().message.clone())); }
        if amount_converted.is_err() { input_errors.push((1, amount_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::DynamicImage{data, change_id:_}) = image_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(amount)) = amount_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:data.huerotate((amount * 360.0) as i32), change_id:get_id() }},
            ],
        })
    }
}
