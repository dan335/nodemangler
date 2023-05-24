use image::RgbaImage;

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType};
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

pub async fn image_from_url(inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
    let start_time = Instant::now();

    //let Value::ImageFormat(image_format) = inputs[1].get_value() else { {return Err(OperationError{message:"Not supported.".to_string()});} };

    let value = match &inputs[0].get_value() {
        Value::String(url) => {
            if let Ok(image_response) = reqwest::get(url).await {
                if let Ok(image_bytes) = image_response.bytes().await {
                    if let Ok(image) = image::load_from_memory(&image_bytes) {
                        Value::DynamicImage(image)
                        // match image_format {
                        //     crate::value::ImageFormat::ImageRgba32F => {
                        //         Value::Rgba32FImage(image.to_rgba32f())
                        //     }
                        //     crate::value::ImageFormat::ImageRgb32F => {
                        //         Value::Rgb32FImage(image.to_rgb32f())
                        //     }
                        //     crate::value::ImageFormat::ImageRgba16 => {
                        //         Value::Rgba16Image(image.to_rgba16())
                        //     }
                        //     crate::value::ImageFormat::ImageRgb16 => {
                        //         Value::Rgb16Image(image.to_rgb16())
                        //     }
                        //     crate::value::ImageFormat::ImageGrayA16 => {
                        //         Value::GrayAlpha16Image(image.to_luma_alpha16())
                        //     }
                        //     crate::value::ImageFormat::ImageGray16 => {
                        //         Value::Gray16Image(image.to_luma16())
                        //     }
                        //     crate::value::ImageFormat::ImageRgba8 => {
                        //         Value::RgbaImage(image.to_rgba8())
                        //     }
                        //     crate::value::ImageFormat::ImageRgb8 => {
                        //         Value::RgbImage(image.to_rgb8())
                        //     }
                        //     crate::value::ImageFormat::ImageGrayA8 => {
                        //         Value::GrayAlphaImage(image.to_luma_alpha8())
                        //     }
                        //     crate::value::ImageFormat::ImageGray8 => {
                        //         Value::GrayImage(image.to_luma8())
                        //     }
                        // }
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
        index: 0,
        value,
        time: Instant::now().duration_since(start_time),
    };

    Ok(vec![node_output_message])
}
