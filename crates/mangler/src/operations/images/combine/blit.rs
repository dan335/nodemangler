use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let background_converted = inputs[0].value.try_convert_to(ValueType::DynamicImage);
        let foreground_converted = inputs[1].value.try_convert_to(ValueType::DynamicImage);
        let position_x_converted = inputs[2].value.try_convert_to(ValueType::Integer);
        let position_y_converted = inputs[3].value.try_convert_to(ValueType::Integer);

        // gather errors
        if background_converted.is_err() { input_errors.push((0, background_converted.as_ref().err().unwrap().message.clone())); }
        if foreground_converted.is_err() { input_errors.push((1, foreground_converted.as_ref().err().unwrap().message.clone())); }
        if position_x_converted.is_err() { input_errors.push((2, position_x_converted.as_ref().err().unwrap().message.clone())); }
        if position_y_converted.is_err() { input_errors.push((3, position_y_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::DynamicImage{data:background_arc, change_id:_}) = background_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::DynamicImage{data:foreground, change_id:_}) = foreground_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(x)) = position_x_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(y)) = position_y_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        let mut background = Arc::try_unwrap(background_arc).unwrap_or_else(|a| (*a).clone());
        image::imageops::overlay(&mut background, &*foreground, x as i64, y as i64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(background), change_id:get_id() }},
            ],
        })
    }
}
