use image::RgbaImage;
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
pub struct OpImageInputUrl {}

impl OpImageInputUrl {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from url".to_string(),
            description: "Grabs an image from a url.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("url".to_string(), Value::String("https://i.imgur.com/3aDSTiBl.jpg".to_string()), Some(InputSettings::MultiLineText), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(i32::default()), None),
            Output::new("height".to_string(), Value::Integer(i32::default()), None),
            Output::new("url".to_string(), Value::String("".to_string()), None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let url_converted = convert_input(inputs, 0, ValueType::String, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::String(url) = url_converted.unwrap() else { unreachable!() };

        // run node
        if let Ok(image_response) =  reqwest::get(url.clone()).await {
            if let Ok(image_bytes) = image_response.bytes().await {
                if let Ok(image) = image::load_from_memory(&image_bytes) {
                    let width = image.width() as i32;
                    let height = image.height() as i32;

                    Ok(OperationResponse {
                        time: Instant::now().duration_since(start_time), 
                        responses: vec![
                            OutputResponse { value: Value::DynamicImage { data: Arc::new(image), change_id: get_id() } },
                            OutputResponse { value: Value::Integer(width) },
                            OutputResponse { value: Value::Integer(height) },
                            OutputResponse { value: Value::String(url) },
                        ],
                    })
                } else {
                    return Err(OperationError{ input_errors, node_error: Some("Format not supported.".to_string())  });
                }
            } else {
                return Err(OperationError{ input_errors, node_error: Some("Could not parse into bytes.".to_string())  });
            }
        } else {
            return Err(OperationError{ input_errors, node_error: Some("Error getting url.".to_string())  });
        }

        
    }
}
