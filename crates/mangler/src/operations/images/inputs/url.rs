use image::RgbaImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
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

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut width = 0;
        let mut height = 0;
        let mut u = "".to_string();

        let value = match &inputs[0].value {
            Value::String(url) => {
                u = url.clone();
                if let Ok(image_response) =  reqwest::get(url).await {
                    if let Ok(image_bytes) = image_response.bytes().await {
                        if let Ok(image) = image::load_from_memory(&image_bytes) {
                            width = image.width();
                            height = image.height();
                            Value::DynamicImage { data:image, change_id: get_id() }
                        } else {
                            return Err(OperationError{ message: "Format not supported".to_string() });
                        }
                    } else {
                        return Err(OperationError{ message: "Could not parse into bytes.".to_string() });
                    }
                } else {
                    return Err(OperationError{ message: "Error getting url.".to_string() });
                }
            }

            _ => return Err(OperationError{ message: "Unable to convert format to url.".to_string() })
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: value },
                OutputResponse { value: Value::Integer(width as i32) },
                OutputResponse { value: Value::Integer(height as i32) },
                OutputResponse { value: Value::String(u) },
            ],
        })
    }
}
