use image::{RgbaImage, DynamicImage};

use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{OperationError, OperationResponse, ConnectionSettings, UiType};
use crate::value::{Value, ValueType};
use core::panic;
use std::time::Instant;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Resize".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "image".to_string(),
            default_value: Value::RgbaImage(RgbaImage::new(32, 32)),
            valid_types: vec![
                ValueType::Rgba32FImage,
                ValueType::RgbaImage,
                ValueType::GrayImage
            ],
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
            default_value: Value::RgbaImage(RgbaImage::new(32, 32)),
            valid_types: vec![
                ValueType::Rgba32FImage,
                ValueType::RgbaImage,
                ValueType::GrayImage
            ],
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

pub async fn image_resize(inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
    let start_time = Instant::now();

    let Ok(Value::Integer(mut width)) = inputs[1].get_value().try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
    let Ok(Value::Integer(mut height)) = inputs[2].get_value().try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
    // let Value::Integer(mut width) = inputs[1].get_value() else { panic!("not suported")};
    // let Value::Integer(mut height) = inputs[2].get_value() else { panic!("not suported")};
    let Value::Bool(auto_width) = inputs[3].get_value() else { panic!("not suported")};
    let Value::Bool(auto_height) = inputs[4].get_value() else { panic!("not suported")};
    let Value::FilterType(filter_type) = inputs[5].get_value() else { panic!("not suported")};

    width = width.max(1);
    height = height.max(1);

    let dynamic_image = match &inputs[0].get_value() {
        Value::Rgba32FImage(image) => DynamicImage::ImageRgba32F(image.clone()),
        Value::Rgb32FImage(image) => DynamicImage::ImageRgb32F(image.clone()),
        Value::Rgba16Image(image) => DynamicImage::ImageRgba16(image.clone()),
        Value::Rgb16Image(image) => DynamicImage::ImageRgb16(image.clone()),
        Value::GrayAlpha16Image(image) => DynamicImage::ImageLumaA16(image.clone()),
        Value::Gray16Image(image) => DynamicImage::ImageLuma16(image.clone()),
        Value::RgbaImage(image) => DynamicImage::ImageRgba8(image.clone()),
        Value::RgbImage(image) => DynamicImage::ImageRgb8(image.clone()),
        Value::GrayAlphaImage(image) => DynamicImage::ImageLumaA8(image.clone()),
        Value::GrayImage(image) => DynamicImage::ImageLuma8(image.clone()),
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Decimal(_)
        | Value::String(_)
        | Value::FilterType(_)
        | Value::ImageFormat(_) => {
            panic!("Unsupported.")
        }
        Value::UiButton(_) => todo!(),
    };

    let resized = dynamic_image.resize_exact(width as u32, height as u32, *filter_type);

    let value_0 = match inputs[0].get_value().clone().value_type() {
        ValueType::Rgba32FImage => Value::Rgba32FImage(resized.to_rgba32f()),
        ValueType::Rgb32FImage => Value::Rgb32FImage(resized.to_rgb32f()),
        ValueType::Rgba16Image => Value::Rgba16Image(resized.to_rgba16()),
        ValueType::Rgb16Image => Value::Rgb16Image(resized.to_rgb16()),
        ValueType::GrayAlpha16Image => Value::GrayAlpha16Image(resized.to_luma_alpha16()),
        ValueType::Gray16Image => Value::Gray16Image(resized.to_luma16()),
        ValueType::RgbaImage => Value::RgbaImage(resized.to_rgba8()),
        ValueType::RgbImage => Value::RgbImage(resized.to_rgb8()),
        ValueType::GrayAlphaImage => Value::GrayAlphaImage(resized.to_luma_alpha8()),
        ValueType::GrayImage => Value::GrayImage(resized.to_luma8()),
        ValueType::Bool
        | ValueType::Integer
        | ValueType::Decimal
        | ValueType::String
        | ValueType::FilterType
        | ValueType::ImageFormat => {
            return Err(OperationError{ message: "Format not supported.".to_string() });
        }
        ValueType::UiButton => todo!(),
    };

    let value_1 = Value::Integer(resized.width() as i32);
    let value_2 = Value::Integer(resized.height() as i32);

    let time = Instant::now().duration_since(start_time);

    let mut node_output_messages: Vec<OperationResponse> = Vec::new();

    node_output_messages.push(OperationResponse {
        index: 0,
        value: value_0,
        time,
    });

    node_output_messages.push(OperationResponse {
        index: 1,
        value: value_1,
        time,
    });

    node_output_messages.push(OperationResponse {
        index: 2,
        value: value_2,
        time,
    });

    Ok(node_output_messages)
}
