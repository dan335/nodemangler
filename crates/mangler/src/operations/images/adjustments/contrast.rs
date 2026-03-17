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
pub struct OpImageAdjustmentContrast {}

impl OpImageAdjustmentContrast {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "contrast".to_string(),
            description: "Adjusts the contrast of an image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None)
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
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };

        // run node
        let adjusted = data.adjust_contrast(amount);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
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
    async fn test_contrast() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("amount".to_string(), Value::Decimal(1.5), None, None),
        ];
        let result = OpImageAdjustmentContrast::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_contrast_settings() {
        let s = OpImageAdjustmentContrast::settings();
        assert_eq!(s.name, "contrast");
        assert_eq!(OpImageAdjustmentContrast::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentContrast::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_contrast_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentContrast::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 contrast failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_contrast_zero_amount() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageAdjustmentContrast::run(&mut inputs).await;
        assert!(result.is_ok(), "zero contrast failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_contrast_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 8), None, None),
            Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentContrast::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
