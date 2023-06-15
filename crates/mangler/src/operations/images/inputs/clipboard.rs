use image::{RgbaImage, ImageBuffer};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use arboard::Clipboard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageInputClipboard {}

impl OperationImageInputClipboard {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image from clipboard".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("copy to clipboard".to_string(), Value::Trigger, InputSettings::None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32)), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(i32::default()), None),
            Output::new("height".to_string(), Value::Integer(i32::default()), None),
        ]
    }

    pub async fn run(_inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut width = 0;
        let mut height = 0;
        let mut img = None;

        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(image_bytes) = clipboard.get_image() {
                let image_option: Option<RgbaImage> = ImageBuffer::from_raw(
                    image_bytes.width.try_into().unwrap(),
                    image_bytes.height.try_into().unwrap(),
                    image_bytes.bytes.into_owned(),
                );
                
                if let Some(image) = image_option{
                    width = image.width();
                    height = image.height();
                    img = Some(Value::DynamicImage{ data:image::DynamicImage::ImageRgba8(image), change_id:get_id() });
                } 
            }
        }

        if let Some(value) = img {
            Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: value },
                    OutputResponse { value: Value::Integer(width as i32) },
                    OutputResponse { value: Value::Integer(height as i32) },
                ],
            })
        } else {
            Err(OperationError { message: "Error grabbing image from clipboard.".to_string() })
        }
    }
}
