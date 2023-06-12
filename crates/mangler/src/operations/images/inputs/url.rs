use image::RgbaImage;
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageInputUrl {}

impl OperationImageInputUrl {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image from url".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("url".to_string(), Value::String("https://i.imgur.com/3aDSTiBl.jpg".to_string()), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id() }, None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let value = match &inputs[0].value {
            Value::String(url) => {
                if let Ok(image_response) =  reqwest::get(url).await {
                    if let Ok(image_bytes) = image_response.bytes().await {
                        if let Ok(image) = image::load_from_memory(&image_bytes) {
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
            responses: vec![OutputResponse {
                value: value,
            }],
        })
    }
}
