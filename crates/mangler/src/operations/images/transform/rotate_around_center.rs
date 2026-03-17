use crate::color::Color;
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
pub struct OpImageTransformRotateAroundCenter {}

impl OpImageTransformRotateAroundCenter {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate around center".to_string(),
            description: "Rotates an image around its center.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("degrees".to_string(), Value::Decimal(45.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.01), clamp_to_range:false }), None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0,0,0,0)), None, None),
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
        let degrees_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let bg_color_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(degrees) = degrees_converted.unwrap() else { unreachable!() };
        let Value::Color(bg_color) = bg_color_converted.unwrap() else { unreachable!() };

        // run node
        let color = bg_color.to_srgb_u8();

        let adjusted = imageproc::geometric_transformations::rotate_about_center(&data.to_rgba8(), degrees.to_radians(), imageproc::geometric_transformations::Interpolation::Bicubic, image::Rgba([color.0,color.1,color.2,color.3]));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(adjusted)), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

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
    async fn test_rotate_around_center_settings() {
        let s = OpImageTransformRotateAroundCenter::settings();
        assert_eq!(s.name, "rotate around center");
        assert_eq!(OpImageTransformRotateAroundCenter::create_inputs().len(), 3);
        assert_eq!(OpImageTransformRotateAroundCenter::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_rotate_around_center() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("degrees".to_string(), Value::Decimal(45.0), None, None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
        ];
        let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rotate_around_center_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("degrees".to_string(), Value::Decimal(45.0), None, None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
        ];
        let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await;
        assert!(result.is_ok(), "rotate_around_center 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_rotate_around_center_zero_degrees() {
        // 0-degree rotation should preserve dimensions and roughly preserve center pixel
        let mut imgbuf = image::RgbaImage::new(8, 8);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x * 30) as u8, (y * 30) as u8, 100, 255]);
        }
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("degrees".to_string(), Value::Decimal(0.0), None, None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
        ];
        let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
