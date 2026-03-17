use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformFlipVertical {}

impl OpImageTransformFlipVertical {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flip vertical".to_string(),
            description: "Flips an image vertically.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        image::imageops::flip_vertical_in_place(&mut data_inner);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(data_inner), change_id:get_id() }},
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
    async fn test_flip_vertical_settings() {
        let s = OpImageTransformFlipVertical::settings();
        assert_eq!(s.name, "flip vertical");
        assert_eq!(OpImageTransformFlipVertical::create_inputs().len(), 1);
        assert_eq!(OpImageTransformFlipVertical::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_flip_vertical() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_flip_vertical_1x1() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
        let result = OpImageTransformFlipVertical::run(&mut inputs).await;
        assert!(result.is_ok(), "flip_vertical 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_flip_vertical_preserves_dimensions() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
        let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_flip_vertical_reverses_rows() {
        // Top pixel should move to bottom after flip
        let mut imgbuf = image::RgbaImage::new(4, 4);
        imgbuf.put_pixel(0, 0, image::Rgba([255u8, 0, 0, 255]));
        imgbuf.put_pixel(0, 3, image::Rgba([0u8, 0, 255, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None)];
        let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let top = data.to_rgba8().get_pixel(0, 0).0;
                // After vertical flip, row 3 becomes row 0
                assert_eq!(top[2], 255, "blue should be at top after flip, got {:?}", top);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
