use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::transform::warp::bilinear_sample_rgba;
use crate::output::Output;
use crate::value::Value;
use image::DynamicImage;
use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentNonUniformBlur {}

impl OpImageAdjustmentNonUniformBlur {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "non-uniform blur".to_string(),
            description: "Blurs with per-pixel intensity controlled by a grayscale map.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blur map".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("max intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 50.0), step_by: Some(0.5), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(16), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None),
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

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let blur_map_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let max_intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: blur_map_data, change_id: _ } = blur_map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max_intensity) = max_intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let max_intensity = max_intensity.max(0.0) as f32;

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();

        // resize blur map to match source if needed
        let blur_map_resized = if blur_map_data.width() != width || blur_map_data.height() != height {
            blur_map_data.resize_exact(width, height, FilterType::Lanczos3)
        } else {
            (*blur_map_data).clone()
        };
        let blur_map_rgba = blur_map_resized.to_rgba8();

        // precompute concentric disc sample offsets (fixed pattern)
        // generate points in concentric rings for a unit disc
        let mut offsets: Vec<(f32, f32)> = Vec::with_capacity(samples as usize);
        if samples == 1 {
            offsets.push((0.0, 0.0));
        } else {
            // distribute points in rings using a sunflower/Vogel disc pattern
            let golden_angle: f32 = 2.399_963_2; // pi * (3 - sqrt(5))
            for i in 0..samples {
                let r = (i as f32 + 0.5).sqrt() / (samples as f32).sqrt();
                let theta = i as f32 * golden_angle;
                offsets.push((r * theta.cos(), r * theta.sin()));
            }
        }

        let mut output_buf = image::RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                // read blur map luminance for per-pixel radius
                let map_px = blur_map_rgba.get_pixel(x, y);
                let luminance = 0.299 * (map_px[0] as f32 / 255.0)
                    + 0.587 * (map_px[1] as f32 / 255.0)
                    + 0.114 * (map_px[2] as f32 / 255.0);
                let radius = luminance * max_intensity;

                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for &(ox, oy) in &offsets {
                    let sx = x as f32 + ox * radius;
                    let sy = y as f32 + oy * radius;
                    let pixel = bilinear_sample_rgba(&rgba, sx, sy);
                    r_sum += pixel[0] as f64;
                    g_sum += pixel[1] as f64;
                    b_sum += pixel[2] as f64;
                    a_sum += pixel[3] as f64;
                }

                let count = samples as f64;
                output_buf.put_pixel(x, y, image::Rgba([
                    (r_sum / count) as u8,
                    (g_sum / count) as u8,
                    (b_sum / count) as u8,
                    (a_sum / count) as u8,
                ]));
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(output_buf)), change_id: get_id() } },
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
    async fn test_non_uniform_blur_settings() {
        let s = OpImageAdjustmentNonUniformBlur::settings();
        assert_eq!(s.name, "non-uniform blur");
        assert_eq!(OpImageAdjustmentNonUniformBlur::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentNonUniformBlur::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_non_uniform_blur_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let blur_map = {
            let bm = image::RgbaImage::from_pixel(1, 1, image::Rgba([128u8, 128, 128, 255]));
            Arc::new(DynamicImage::ImageRgba8(bm))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("blur map".to_string(), Value::DynamicImage { data: blur_map, change_id: get_id() }, None, None),
            Input::new("max intensity".to_string(), Value::Decimal(5.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentNonUniformBlur::run(&mut inputs).await;
        assert!(result.is_ok(), "non_uniform_blur 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_non_uniform_blur_zero_intensity() {
        // With a black blur map (zero intensity per-pixel), output should match input
        let uniform = {
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([100u8, 100, 100, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        };
        let black_map = {
            let bm = image::RgbaImage::from_pixel(8, 8, image::Rgba([0u8, 0, 0, 255]));
            Arc::new(DynamicImage::ImageRgba8(bm))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
            Input::new("blur map".to_string(), Value::DynamicImage { data: black_map, change_id: get_id() }, None, None),
            Input::new("max intensity".to_string(), Value::Decimal(20.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageAdjustmentNonUniformBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(4, 4).0;
                assert!((p[0] as i32 - 100).abs() <= 2, "zero-blur map: expected ~100, got {}", p[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_non_uniform_blur_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("blur map".to_string(), image_input(8, 8), None, None),
            Input::new("max intensity".to_string(), Value::Decimal(5.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageAdjustmentNonUniformBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
