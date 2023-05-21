use image::RgbaImage;
use tokio::sync::mpsc::Sender;
use crate::NodeOutputChangedMessage;
use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
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
        ConnectionSettings {
            name: "image format".to_string(),
            default_value: Value::ImageFormat(crate::value::ImageFormat::ImageRgba8),
            valid_types: vec![ValueType::String],
            ui_type: Some(UiType::DragValue),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "image".to_string(),
        default_value: Value::ImageRgba8(RgbaImage::new(32, 32)),
        valid_types: vec![ValueType::ImageRgba8],
        ui_type: None,
    },];
}


pub async fn image_from_url(node_id: &String, inputs: &[Input]) -> Vec<NodeOutputChangedMessage> {
    let start_time = Instant::now();

    let Value::ImageFormat(image_format) = inputs[1].get_value() else { panic!("not suported")};

    let value = match &inputs[0].get_value() {
        Value::String(url) => {
            if let Ok(image_response) = reqwest::get(url).await {
                if let Ok(image_bytes) = image_response.bytes().await {
                    if let Ok(image) = image::load_from_memory(&image_bytes) {
                        match image_format {
                            crate::value::ImageFormat::ImageRgba32F => Value::ImageRgba32F(image.to_rgba32f()),
                            crate::value::ImageFormat::ImageRgb32F => Value::ImageRgb32F(image.to_rgb32f()),
                            crate::value::ImageFormat::ImageRgba16 => Value::ImageRgba16(image.to_rgba16()),
                            crate::value::ImageFormat::ImageRgb16 => Value::ImageRgb16(image.to_rgb16()),
                            crate::value::ImageFormat::ImageGrayA16 => Value::ImageGrayA16(image.to_luma_alpha16()),
                            crate::value::ImageFormat::ImageGray16 => Value::ImageGray16(image.to_luma16()),
                            crate::value::ImageFormat::ImageRgba8 => Value::ImageRgba8(image.to_rgba8()),
                            crate::value::ImageFormat::ImageRgb8 => Value::ImageRgb8(image.to_rgb8()),
                            crate::value::ImageFormat::ImageGrayA8 => Value::ImageGrayA8(image.to_luma_alpha8()),
                            crate::value::ImageFormat::ImageGray8 => Value::ImageGray8(image.to_luma8()),
                        }
                    } else {
                        println!("format not supported.");
                        OUTPUT_SETTINGS[0].default_value.clone()
                    }                        
                } else {
                    println!("could not parse into bytes.");
                    OUTPUT_SETTINGS[0].default_value.clone()
                }
            } else {
                println!("error fetching url.");
                OUTPUT_SETTINGS[0].default_value.clone()
            }
        },

        _ => panic!("Unable to convert formats to url."),
    };

    let node_output_message = NodeOutputChangedMessage {
        node_id: node_id.clone(),
        output_index: 0,
        value_type: value.value_type(),
        value,
        time: Instant::now().duration_since(start_time),
        thumbnail: None,
    };

    vec![node_output_message]
}
