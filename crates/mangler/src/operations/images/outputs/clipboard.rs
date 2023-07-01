use image::RgbaImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use arboard::{Clipboard, ImageData};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageOutputClipboard {}

impl OperationImageOutputClipboard {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image to clipboard".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id() }, InputSettings::None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        
        let Value::DynamicImage{data, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        //if let Some(rgba8) = data.to_rgba8() {
            let rgba8 = data.to_rgba8();
            //if let Some(flat_samples) = rgba8.as_flat_samples() {
                let image_data = ImageData {
                    width: data.width() as usize,
                    height: data.height() as usize,
                    bytes: std::borrow::Cow::Borrowed( rgba8.as_flat_samples().samples)
                };
                
                if let Ok(mut clipboard) = Clipboard::new() {
                    if let Ok(_) = clipboard.set_image(image_data) {
                        Ok(OperationResponse {
                            time: Instant::now().duration_since(start_time),
                            responses: vec![],
                        })
                    } else {
                        Err(OperationError { message: "Unable to convert to path.".to_string() })
                    }
                } else {
                    Err(OperationError { message: "Unable to convert to path.".to_string() })
                }
            // } else {
            //     Err(OperationError { message: "Unable to convert to path.".to_string() })
            // }
        // } else {
        //     Err(OperationError { message: "Unable to convert to path.".to_string() })
        // }
    }
}
