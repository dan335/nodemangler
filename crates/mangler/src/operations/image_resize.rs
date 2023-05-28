use image::{RgbaImage, DynamicImage};

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType, OutputResponse};
use crate::value::{Value, ValueType};
use std::time::Instant;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Resize".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "image".to_string(),
            default_value: Value::DynamicImage(image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32))),
            valid_types: vec![
                ValueType::DynamicImage,
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
            default_value: Value::DynamicImage(image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32))),
            valid_types: vec![
                ValueType::DynamicImage,
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

pub async fn image_resize(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
    let start_time = Instant::now();

    let Ok(Value::Integer(mut width)) = inputs[1].get_value().try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
    let Ok(Value::Integer(mut height)) = inputs[2].get_value().try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
    let Value::Bool(auto_width) = inputs[3].get_value() else { return Err(OperationError{message:"Not supported".to_string()}); };
    let Value::Bool(auto_height) = inputs[4].get_value() else { return Err(OperationError{message:"Not supported".to_string()}); };
    let Value::FilterType(filter_type) = inputs[5].get_value() else { return Err(OperationError{message:"Not supported".to_string()}); };

    width = width.max(1);
    height = height.max(1);

    let Value::DynamicImage(image) = inputs[0].get_value() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

    let resized = image.resize_exact(width as u32, height as u32, *filter_type);
    let value_1 = Value::Integer(resized.width() as i32);
    let value_2 = Value::Integer(resized.height() as i32);

    let time = Instant::now().duration_since(start_time);

    let mut node_output_messages: Vec<OperationResponse> = Vec::new();

    Ok(OperationResponse {
        outputs: vec![
            OutputResponse { value: Value::DynamicImage(resized) },
            OutputResponse { value: value_1 },
            OutputResponse { value: value_2 },
        ],
        time,
    })
}
