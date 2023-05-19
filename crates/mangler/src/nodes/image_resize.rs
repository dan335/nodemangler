use image::{RgbaImage, DynamicImage};

use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use core::panic;
use std::any::Any;
use std::time::{Duration, Instant};
use std::thread;

use super::operation::OperationResponse;

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

pub fn image_resize(inputs: &[Input], outputs: &mut [Output]) -> OperationResponse {
    let start_time = Instant::now();

    let mut response = OperationResponse::new();

    let Value::Integer(mut width) = inputs[1].get_value() else { panic!("not suported")};
    let Value::Integer(mut height) = inputs[2].get_value() else { panic!("not suported")};
    let Value::Bool(auto_width) = inputs[3].get_value() else { panic!("not suported")};
    let Value::Bool(auto_height) = inputs[4].get_value() else { panic!("not suported")};
    let Value::FilterType(filter_type) = inputs[5].get_value() else { panic!("not suported")};

    width = width.max(1);
    height = height.max(1);

    let dynamic_image = match &inputs[0].get_value() {
        Value::ImageRgba32F(image) => DynamicImage::ImageRgba32F(image.clone()),
        Value::ImageRgb32F(image) => DynamicImage::ImageRgb32F(image.clone()),
        Value::ImageRgba16(image) => DynamicImage::ImageRgba16(image.clone()),
        Value::ImageRgb16(image) => DynamicImage::ImageRgb16(image.clone()),
        Value::ImageGrayA16(image) => DynamicImage::ImageLumaA16(image.clone()),
        Value::ImageGray16(image) => DynamicImage::ImageLuma16(image.clone()),
        Value::ImageRgba8(image) => DynamicImage::ImageRgba8(image.clone()),
        Value::ImageRgb8(image) => DynamicImage::ImageRgb8(image.clone()),
        Value::ImageGrayA8(image) => DynamicImage::ImageLumaA8(image.clone()),
        Value::ImageGray8(image) => DynamicImage::ImageLuma8(image.clone()),
        Value::Bool(_) |
        Value::Integer(_) |
        Value::Decimal(_) |
        Value::String(_) |
        Value::FilterType(_) |
        Value::ImageFormat(_) => { panic!("Unsupported.") },
    };

    let resized = dynamic_image.resize_exact(width as u32, height as u32, *filter_type);

    response.output_values.push(match inputs[0].get_value().clone().value_type() {
        ValueType::ImageRgba32F => Value::ImageRgba32F(resized.to_rgba32f()),
        ValueType::ImageRgb32F => Value::ImageRgb32F(resized.to_rgb32f()),
        ValueType::ImageRgba16 => Value::ImageRgba16(resized.to_rgba16()),
        ValueType::ImageRgb16 => Value::ImageRgb16(resized.to_rgb16()),
        ValueType::ImageGrayA16 => Value::ImageGrayA16(resized.to_luma_alpha16()),
        ValueType::ImageGray16 => Value::ImageGray16(resized.to_luma16()),
        ValueType::ImageRgba8 => Value::ImageRgba8(resized.to_rgba8()),
        ValueType::ImageRgb8 => Value::ImageRgb8(resized.to_rgb8()),
        ValueType::ImageGrayA8 => Value::ImageGrayA8(resized.to_luma_alpha8()),
        ValueType::ImageGray8 => Value::ImageGray8(resized.to_luma8()),
        ValueType::Bool |
        ValueType::Integer |
        ValueType::Decimal |
        ValueType::String |
        ValueType::FilterType |
        ValueType::ImageFormat => { panic!("Unsupported.") }
    });
    
    outputs[1].value = Value::Integer(resized.width() as i32);
    outputs[2].value = Value::Integer(resized.height() as i32);

    response.time = Instant::now().duration_since(start_time);
    response
}