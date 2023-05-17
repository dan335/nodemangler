use image::{RgbaImage, DynamicImage};

use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use core::panic;
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Resize".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "image".to_string(),
            default_value: Value::ImageRgba8(RgbaImage::new(32, 32)),
            valid_types: vec![ValueType::ImageRgba32F, ValueType::ImageRgba8, ValueType::ImageGray8],
            ui_type: None,
        },
        ConnectionSettings {
            name: "width".to_string(),
            default_value: Value::Integer(32),
            valid_types: vec![ValueType::Integer],
            ui_type: Some(UiType::DragValue),
        },
        ConnectionSettings {
            name: "height".to_string(),
            default_value: Value::Integer(32),
            valid_types: vec![ValueType::Integer],
            ui_type: Some(UiType::DragValue),
        },
        ConnectionSettings {
            name: "auto width".to_string(),
            default_value: Value::Bool(false),
            valid_types: vec![ValueType::Bool],
            ui_type: Some(UiType::Checkbox),
        },
        ConnectionSettings {
            name: "auto height".to_string(),
            default_value: Value::Bool(false),
            valid_types: vec![ValueType::Bool],
            ui_type: Some(UiType::Checkbox),
        },
        ConnectionSettings {
            name: "filter type".to_string(),
            default_value: Value::FilterType(image::imageops::FilterType::Gaussian),
            valid_types: vec![ValueType::Integer, ValueType::FilterType],
            ui_type: Some(UiType::ComboBox),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "image".to_string(),
            default_value: Value::ImageRgba8(RgbaImage::new(32, 32)),
            valid_types: vec![ValueType::ImageRgba32F, ValueType::ImageRgba8, ValueType::ImageGray8],
            ui_type: None,
        },
        ConnectionSettings {
            name: "width".to_string(),
            default_value: Value::Integer(32),
            valid_types: vec![ValueType::Integer],
            ui_type: None,
        },
        ConnectionSettings {
            name: "height".to_string(),
            default_value: Value::Integer(32),
            valid_types: vec![ValueType::Integer],
            ui_type: None,
        },
    ];
}

#[derive(Debug, Clone, Default)]
pub struct ImageResize {}

impl ImageResize {
    pub fn run(&mut self, inputs: &[Input], outputs: &mut [Output]) -> Duration {
        let start_time = Instant::now();

        let Value::Integer(width) = inputs[1].value else { panic!("not suported")};
        let Value::Integer(height) = inputs[2].value else { panic!("not suported")};
        let Value::Bool(auto_width) = inputs[3].value else { panic!("not suported")};
        let Value::Bool(auto_height) = inputs[4].value else { panic!("not suported")};
        let Value::FilterType(filter_type) = inputs[5].value else { panic!("not suported")};

        outputs[0].value = match &inputs[0].value {
            Value::ImageRgba32F(image) => {
                let resized = DynamicImage::ImageRgba32F(image.clone()).resize(width.max(1) as u32, height.max(1) as u32, filter_type).to_rgba32f();
                Value::ImageRgba32F(resized)
            },
            Value::ImageRgba8(image) => {
                let resized = DynamicImage::ImageRgba8(image.clone()).resize(width.max(1) as u32, height.max(1) as u32, filter_type).to_rgba8();
                Value::ImageRgba8(resized)
            },
            Value::ImageGray8(image) => {
                let resized = DynamicImage::ImageLuma8(image.clone()).resize(width.max(1) as u32, height.max(1) as u32, filter_type).to_luma8();
                Value::ImageGray8(resized)
            },
            _ => panic!("unsupported"),
        };

        outputs[1].value = Value::Integer(width);
        outputs[2].value = Value::Integer(height);

        Instant::now().duration_since(start_time)
    }
}
