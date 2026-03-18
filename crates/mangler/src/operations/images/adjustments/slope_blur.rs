//! Slope blur operation for images.
//!
//! Blurs the image along directions derived from the gradient of a separate
//! grayscale slope map. The gradient direction at each pixel determines the
//! blur direction, creating an effect similar to paint being smeared downhill.

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

/// Slope blur operation that blurs along gradient directions derived from a slope map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSlopeBlur {}

impl OpImageAdjustmentSlopeBlur {
    /// Returns the node metadata (name and description) for the slope blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "slope blur".to_string(),
            description: "Blurs along directions determined by a grayscale slope map.".to_string(),
        }
    }

    /// Creates the input ports: source image, slope map (grayscale gradient source),
    /// intensity (pixel spread), and number of samples.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("slope map".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(0.5), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None),
        ]
    }

    /// Creates the output port: the slope-blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the slope blur. Computes per-pixel gradient direction from the slope map
    /// using finite differences, then averages bilinear samples along that direction.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let slope_map_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: slope_data, change_id: _ } = slope_map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let intensity = intensity.max(0.0) as f32;

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();

        // resize slope map to match source if needed
        let slope_resized = if slope_data.width() != width || slope_data.height() != height {
            slope_data.resize_exact(width, height, FilterType::Lanczos3)
        } else {
            (*slope_data).clone()
        };
        let slope_rgba = slope_resized.to_rgba8();

        // helper: get luminance from slope map pixel
        let luminance_at = |x: u32, y: u32| -> f32 {
            let px = slope_rgba.get_pixel(x.min(width - 1), y.min(height - 1));
            0.299 * (px[0] as f32 / 255.0) + 0.587 * (px[1] as f32 / 255.0) + 0.114 * (px[2] as f32 / 255.0)
        };

        let mut output_buf = image::RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                // compute gradient direction from slope map (sobel-like)
                let x_left = if x > 0 { x - 1 } else { 0 };
                let x_right = if x < width - 1 { x + 1 } else { width - 1 };
                let y_top = if y > 0 { y - 1 } else { 0 };
                let y_bottom = if y < height - 1 { y + 1 } else { height - 1 };

                let grad_x = luminance_at(x_right, y) - luminance_at(x_left, y);
                let grad_y = luminance_at(x, y_bottom) - luminance_at(x, y_top);

                let grad_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
                // Normalize gradient to unit direction; zero gradient means no blur direction
                let (dx, dy) = if grad_len > 1e-6 {
                    (grad_x / grad_len, grad_y / grad_len)
                } else {
                    (0.0, 0.0)
                };

                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for i in 0..samples {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                    let offset = t * intensity;
                    let sx = x as f32 + dx * offset;
                    let sy = y as f32 + dy * offset;
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
    async fn test_slope_blur_settings() {
        let s = OpImageAdjustmentSlopeBlur::settings();
        assert_eq!(s.name, "slope blur");
        assert_eq!(OpImageAdjustmentSlopeBlur::create_inputs().len(), 4);
        assert_eq!(OpImageAdjustmentSlopeBlur::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_slope_blur_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let slope_map = {
            let sm = image::RgbaImage::from_pixel(1, 1, image::Rgba([128u8, 128, 128, 255]));
            Arc::new(DynamicImage::ImageRgba8(sm))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("slope map".to_string(), Value::DynamicImage { data: slope_map, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentSlopeBlur::run(&mut inputs).await;
        assert!(result.is_ok(), "slope_blur 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_slope_blur_uniform_image_unchanged() {
        // Uniform image with uniform slope map → no gradient direction → no movement
        let uniform = {
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([100u8, 100, 100, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        };
        let flat_map = {
            let sm = image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]));
            Arc::new(DynamicImage::ImageRgba8(sm))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
            Input::new("slope map".to_string(), Value::DynamicImage { data: flat_map, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentSlopeBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(4, 4).0;
                assert!((p[0] as i32 - 100).abs() <= 2, "uniform slope blur: expected ~100, got {}", p[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_slope_blur_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("slope map".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentSlopeBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
