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
pub struct OpImageAdjustmentGrayscale {}

impl OpImageAdjustmentGrayscale {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grayscale".to_string(),
            description: "Converts an image to grayscale.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id:get_id() }, None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id:get_id() }, None),
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
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(data.grayscale()), change_id:get_id() }},
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
    async fn test_grayscale() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageAdjustmentGrayscale::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_grayscale_settings() {
        let s = OpImageAdjustmentGrayscale::settings();
        assert_eq!(s.name, "grayscale");
        assert_eq!(OpImageAdjustmentGrayscale::create_inputs().len(), 1);
        assert_eq!(OpImageAdjustmentGrayscale::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_grayscale_1x1() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
        let result = OpImageAdjustmentGrayscale::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 grayscale failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_grayscale_preserves_dimensions() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(16, 8), None, None)];
        let result = OpImageAdjustmentGrayscale::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_grayscale_equal_channels() {
        // After grayscale, R == G == B for all pixels
        let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
        let result = OpImageAdjustmentGrayscale::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba8();
                for px in buf.pixels() {
                    assert_eq!(px[0], px[1], "R != G after grayscale");
                    assert_eq!(px[1], px[2], "G != B after grayscale");
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
