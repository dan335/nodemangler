//! Crop operation for extracting a rectangular sub-region from an image.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Crops an image to a rectangular sub-region defined by position (x, y) and size (width, height).
///
/// Inputs are clamped to valid ranges based on the source image dimensions.
/// Outputs the cropped image along with its actual width and height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformCrop {}

impl OpImageTransformCrop {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "crop".to_string(),
            description: "Crops an image.".to_string(),
        }
    }

    /// Creates the default inputs: source image, x/y position, and width/height of the crop region.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("x".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("y".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
        ]
    }

    /// Creates the default outputs: cropped image, and its width and height as integers.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the crop operation.
    ///
    /// Clamps x, y, width, and height to the source image bounds before cropping.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut x) = x_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut y) = y_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };

        // run node
        // Try to take ownership of the image data; clone if other references exist
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        // Clamp crop parameters to valid image bounds
        x = x.max(0).min(data_inner.width() as i32 - 1);
        y = y.max(0).min(data_inner.height() as i32 - 1);
        width = width.max(1).min(data_inner.width() as i32);
        height = height.max(1).min(data_inner.height() as i32);

        let resized = image::imageops::crop(&mut data_inner, x as u32, y as u32, width as u32, height as u32).to_image();

        let value_width = Value::Integer(resized.width() as i32);
        let value_height = Value::Integer(resized.height() as i32);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(image::DynamicImage::ImageRgba8(resized)), change_id:get_id() }},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::get_id;
    use crate::input::Input;
    use crate::value::Value;
    use image::DynamicImage;
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            *pixel = image::Rgba([r, g, 128, 255]);
        }
        Arc::new(DynamicImage::ImageRgba8(imgbuf))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }

    #[tokio::test]
    async fn test_crop_settings() {
        let s = OpImageTransformCrop::settings();
        assert_eq!(s.name, "crop");
        assert_eq!(OpImageTransformCrop::create_inputs().len(), 5);
        assert_eq!(OpImageTransformCrop::create_outputs().len(), 3);
    }

    #[tokio::test]
    async fn test_crop() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("x".to_string(), Value::Integer(1), None, None),
            Input::new("y".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3);
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_crop_output_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("x".to_string(), Value::Integer(0), None, None),
            Input::new("y".to_string(), Value::Integer(0), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 4);
                assert_eq!(data.height(), 3);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_crop_full_image() {
        // Cropping the full image should give back the same dimensions
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("x".to_string(), Value::Integer(0), None, None),
            Input::new("y".to_string(), Value::Integer(0), None, None),
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
