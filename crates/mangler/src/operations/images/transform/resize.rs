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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformResize {}

impl OpImageTransformResize {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resize".to_string(),
            description: "Resizes an image to a specific size.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("width".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let filter_type_converted = convert_input(inputs, 3, ValueType::FilterType, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::FilterType(filter_type) = filter_type_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);

        let resized = data.resize(width as u32, height as u32, filter_type);

        let value_width = Value::Integer(resized.width() as i32);
        let value_height = Value::Integer(resized.height() as i32);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(resized), change_id:get_id() }},
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
    async fn test_resize_settings() {
        let s = OpImageTransformResize::settings();
        assert_eq!(s.name, "resize");
        assert_eq!(OpImageTransformResize::create_inputs().len(), 4);
        assert_eq!(OpImageTransformResize::create_outputs().len(), 3);
    }

    #[tokio::test]
    async fn test_resize() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
        ];
        let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 3);
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_resize_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("width".to_string(), Value::Integer(1), None, None),
            Input::new("height".to_string(), Value::Integer(1), None, None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
        ];
        let result = OpImageTransformResize::run(&mut inputs).await;
        assert!(result.is_ok(), "resize to 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_resize_upscale() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(16), None, None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
        ];
        let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                // resize (aspect-ratio preserving) may not give exact target
                assert!(data.width() > 0 && data.height() > 0);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
