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
pub struct OpImageTransformDirectionalWarp {}

impl OpImageTransformDirectionalWarp {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "directional warp".to_string(),
            description: "Displaces pixels along a single angle, with intensity driven by a grayscale map.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity map".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.1), clamp_to_range: false }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let map_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let map_img = map_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        let angle_rad = angle.to_radians();
        let dir_x = angle_rad.cos();
        let dir_y = angle_rad.sin();

        for y in 0..h {
            for x in 0..w {
                // Sample intensity map (resize-aware)
                let mx = x as f32 * map_img.width() as f32 / w as f32;
                let my = y as f32 * map_img.height() as f32 / h as f32;
                let mp = super::warp::bilinear_sample_rgba(&map_img, mx, my);

                // Luminance of the map pixel (0..1), centered to -0.5..0.5
                let lum = (mp[0] as f32 * 0.299 + mp[1] as f32 * 0.587 + mp[2] as f32 * 0.114) / 255.0 - 0.5;
                let displacement = lum * intensity;

                let sx = x as f32 + dir_x * displacement;
                let sy = y as f32 + dir_y * displacement;

                let pixel = super::warp::bilinear_sample_rgba(&src, sx, sy);
                output.put_pixel(x, y, image::Rgba(pixel));
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
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

    fn gradient_h_image(w: u32, h: u32) -> Value {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
            let v = (x * 255 / w.max(1)) as u8;
            *pixel = image::Rgba([v, v, v, 255]);
        }
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }
    }

    #[tokio::test]
    async fn test_directional_warp_settings() {
        let s = OpImageTransformDirectionalWarp::settings();
        assert_eq!(s.name, "directional warp");
        assert_eq!(OpImageTransformDirectionalWarp::create_inputs().len(), 4);
        assert_eq!(OpImageTransformDirectionalWarp::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_directional_warp_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("intensity map".to_string(), gradient_h_image(16, 16), None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 16);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_directional_warp_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("intensity map".to_string(), image_input(1, 1), None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageTransformDirectionalWarp::run(&mut inputs).await;
        assert!(result.is_ok(), "directional_warp 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_directional_warp_zero_intensity_passthrough() {
        // With intensity=0, all displacements are 0 → output should equal input
        let uniform = image::RgbaImage::from_pixel(8, 8, image::Rgba([150u8, 200, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(uniform));
        let map = image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]));
        let map_img = Arc::new(DynamicImage::ImageRgba8(map));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("intensity map".to_string(), Value::DynamicImage { data: map_img, change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
            Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(4, 4).0;
                assert_eq!(p, [150u8, 200, 50, 255], "zero intensity should give passthrough");
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_directional_warp_different_angles() {
        // Run at multiple angles to ensure no panics
        for angle in [0.0, 45.0, 90.0, 180.0, 270.0, 360.0] {
            let mut inputs = vec![
                Input::new("image".to_string(), image_input(8, 8), None, None),
                Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
                Input::new("angle".to_string(), Value::Decimal(angle), None, None),
                Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageTransformDirectionalWarp::run(&mut inputs).await;
            assert!(result.is_ok(), "directional_warp at angle {} failed: {:?}", angle, result.err());
        }
    }
}
