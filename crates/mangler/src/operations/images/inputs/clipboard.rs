use image::{RgbaImage, ImageBuffer};
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use arboard::Clipboard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputClipboard {}

impl OpImageInputClipboard {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from clipboard".to_string(),
            description: "Grabs an image from the clipboard.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("copy from clipboard".to_string(), Value::Trigger, None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    pub async fn run(_inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        
        // run node
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
                    img = Some(Value::DynamicImage{ data:Arc::new(image::DynamicImage::ImageRgba8(image)), change_id:get_id() });
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
            Err(OperationError { input_errors: vec![], node_error: Some("Error grabbing clipboard or clipboard is empty.".to_string())  })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_input_settings() {
        let s = OpImageInputClipboard::settings();
        assert!(!s.name.is_empty());
        assert!(!OpImageInputClipboard::create_inputs().is_empty());
        assert!(!OpImageInputClipboard::create_outputs().is_empty());
    }
}
