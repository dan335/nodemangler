use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{OperationError, OperationResponse, ConnectionSettings, UiType};
use crate::value::{Value, ValueType};
use std::time::Instant;

use arboard::Clipboard;
use image::{RgbaImage, ImageBuffer};

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Image from Clipboard".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "Copy from Clipboard".to_string(),
            default_value: Value::UiButton(true),
            valid_types: vec![ValueType::Bool],
            ui_type: Some(UiType::UiButton),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "image".to_string(),
        default_value: Value::RgbaImage(RgbaImage::new(32, 32)),
        valid_types: vec![ValueType::RgbaImage],
        ui_type: None,
    },];
}

pub async fn image_from_clipboard(_inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
    let start_time = Instant::now();

    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(image_bytes) = clipboard.get_image() {
            let image_option: Option<RgbaImage> = ImageBuffer::from_raw(
                image_bytes.width.try_into().unwrap(),
                image_bytes.height.try_into().unwrap(),
                image_bytes.bytes.into_owned(),
            );
            
            if let Some(image) = image_option{
                let node_output_message = OperationResponse {
                    index: 0,
                    value: Value::RgbaImage(image),
                    time: Instant::now().duration_since(start_time),
                };

                return Ok(vec![node_output_message]);
            } 
        }
    }

    Err(OperationError { message: "Error grabbing image from clipboard.".to_string() })
}