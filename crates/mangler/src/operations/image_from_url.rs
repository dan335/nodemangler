use image::RgbaImage;

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType, OutputResponse};
use crate::value::{Value, ValueType};
use std::time::Instant;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Image from URL".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "url".to_string(),
            default_value: Value::String("https://i.imgur.com/3aDSTiBl.jpg".to_string()),
            valid_types: vec![ValueType::String],
            ui_type: Some(UiType::DragValue),
        },
        // ConnectionSettings {
        //     name: "image format".to_string(),
        //     default_value: Value::ImageFormat(crate::value::ImageFormat::ImageRgba8),
        //     valid_types: vec![ValueType::String],
        //     ui_type: Some(UiType::DragValue),
        // },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "image".to_string(),
        default_value: Value::DynamicImage(image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32))),
        valid_types: vec![ValueType::DynamicImage],
        ui_type: None,
    },];
}

pub async fn image_from_url(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
    let start_time = Instant::now();

    //let Value::ImageFormat(image_format) = inputs[1].get_value() else { {return Err(OperationError{message:"Not supported.".to_string()});} };

    let value = match &inputs[0].get_value() {
        Value::String(url) => {
            if let Ok(image_response) = reqwest::get(url).await {
                if let Ok(image_bytes) = image_response.bytes().await {
                    if let Ok(image) = image::load_from_memory(&image_bytes) {
                        Value::DynamicImage(image)
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

    let node_output_message = OperationResponse {
        time: Instant::now().duration_since(start_time),
        outputs: vec![OutputResponse {
            value,
        }]
    };

    Ok(node_output_message)
}
