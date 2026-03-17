use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
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
        let background_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let foreground_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let position_x_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let position_y_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:background_arc, change_id:_} = background_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:foreground, change_id:_} = foreground_converted.unwrap() else { unreachable!() };
        let Value::Integer(x) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(y) = position_y_converted.unwrap() else { unreachable!() };

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
