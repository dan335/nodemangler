use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentLevels{}

impl OpImageAdjustmentLevels {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "levels".to_string(),
            description: "Adjusts black point, white point, and gamma.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("black point".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("white point".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("gamma".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.01), clamp_to_range: true }), None),
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
        let black_point_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let white_point_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let gamma_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(black_point) = black_point_converted.unwrap() else { unreachable!() };
        let Value::Decimal(white_point) = white_point_converted.unwrap() else { unreachable!() };
        let Value::Decimal(gamma) = gamma_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let black_point = black_point as f32;
        let white_point = white_point as f32;
        let range = (white_point - black_point).max(0.001);
        let inv_gamma = (1.0 / gamma) as f32;

        for pixel in buffer.pixels_mut() {
            for c in 0..3 {
                let val = pixel[c];
                let remapped = ((val - black_point) / range).clamp(0.0, 1.0);
                let corrected = remapped.powf(inv_gamma);
                pixel[c] = corrected;
            }
            // alpha unchanged
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

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
    async fn test_levels_settings() {
        let s = OpImageAdjustmentLevels::settings();
        assert_eq!(s.name, "levels");
        assert_eq!(OpImageAdjustmentLevels::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentLevels::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_levels_identity() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("black point".to_string(), Value::Decimal(0.0), None, None),
            Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
            Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_levels_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("black point".to_string(), Value::Decimal(0.0), None, None),
            Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
            Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentLevels::run(&mut inputs).await;
        assert!(result.is_ok(), "levels 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_levels_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("black point".to_string(), Value::Decimal(0.2), None, None),
            Input::new("white point".to_string(), Value::Decimal(0.8), None, None),
            Input::new("gamma".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                for pixel in data.to_rgba32f().pixels() {
                    for c in 0..3 {
                        assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of range: {}", pixel[c]);
                    }
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_levels_crush_blacks() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([64, 64, 64, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("black point".to_string(), Value::Decimal(0.5), None, None),
            Input::new("white point".to_string(), Value::Decimal(1.0), None, None),
            Input::new("gamma".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p[0], 0);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
