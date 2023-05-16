use image::RgbaImage;

use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Image from URL".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "url".to_string(),
        default_value: Value::String("https://i.imgur.com/3aDSTiBl.jpg".to_string()),
        valid_types: vec![ValueType::String],
        ui_type: Some(UiType::DragValue),
    },];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "image".to_string(),
        default_value: Value::ImageRgba8(RgbaImage::new(32, 32)),
        valid_types: vec![ValueType::ImageRgba8],
        ui_type: None,
    },];
}

#[derive(Debug, Clone, Default)]
pub struct ImageFromUrl {}

impl ImageFromUrl {
    pub fn run(&mut self, inputs: &[Input], outputs: &mut [Output]) -> Duration {
        let start_time = Instant::now();

        outputs[0].value = match &inputs[0].value {
            Value::String(url) => {
                if let Ok(image_response) = reqwest::blocking::get(url) {
                    if let Ok(image_bytes) = image_response.bytes() {
                        if let Ok(image) = image::load_from_memory(&image_bytes) {
                            Value::ImageRgba8(image.to_rgba8())
                        } else {
                            OUTPUT_SETTINGS[0].default_value.clone()
                        }                        
                    } else {
                        OUTPUT_SETTINGS[0].default_value.clone()
                    }
                } else {
                    OUTPUT_SETTINGS[0].default_value.clone()
                }
            },

            _ => panic!("Unable to convert formats to url."),
        };

        Instant::now().duration_since(start_time)
    }
}
