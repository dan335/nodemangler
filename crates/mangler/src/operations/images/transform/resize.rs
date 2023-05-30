use crate::value::ValueType;
use image::RgbaImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationImageTransformResize {}

impl OperationImageTransformResize {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resize".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input {
                name: "image".to_string(),
                value: Value::DynamicImage(image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32))),
                connection: None,
                valid_types: vec![],
            },
            Input {
                name: "width".to_string(),
                value:  Value::Integer(32),
                connection: None,
                valid_types: vec![],
            },
            Input {
                name: "height".to_string(),
                value:  Value::Integer(32),
                connection: None,
                valid_types: vec![],
            },
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output {
                name: "image".to_string(),
                value: Value::DynamicImage(image::DynamicImage::ImageRgba8(RgbaImage::new(32, 32))),
                connection: None,
            },
            Output {
                name: "width".to_string(),
                value: Value::Integer(32),
                connection: None,
            },
            Output {
                name: "height".to_string(),
                value: Value::Integer(32),
                connection: None,
            },
        ]
    }

    pub async fn run(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Integer(mut width)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        width = width.max(1);
        height = height.max(1);

        let Value::DynamicImage(image) = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };

        let resized = image.resize_exact(width as u32, height as u32, image::imageops::FilterType::Gaussian);
        let value_width = Value::Integer(resized.width() as i32);
        let value_height = Value::Integer(resized.height() as i32);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage(resized)},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}
