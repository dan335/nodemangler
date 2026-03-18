//! Radial (spin) blur operation for images.
//!
//! Applies a circular motion blur around the image center by sampling
//! pixels at multiple angular offsets at the same radial distance.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::transform::warp::bilinear_sample_rgba;
use crate::output::Output;
use crate::value::Value;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Radial blur operation that creates a circular spin blur effect around the image center.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentRadialBlur {}

impl OpImageAdjustmentRadialBlur {
    /// Returns the node metadata (name and description) for the radial blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "radial blur".to_string(),
            description: "Applies a circular spin blur around the image center.".to_string(),
        }
    }

    /// Creates the input ports: image, spin angle (degrees), and number of samples.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 180.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None),
        ]
    }

    /// Creates the output port: the radially blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the radial blur. For each pixel, computes the angle and distance from
    /// the image center, then averages samples taken at angular offsets around that arc.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let angle_rad = angle.to_radians();

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let mut output_buf = image::RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let base_angle = dy.atan2(dx);
                let dist = (dx * dx + dy * dy).sqrt();

                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for i in 0..samples {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) - 0.5
                    } else {
                        0.0
                    };
                    let sample_angle = base_angle + t * angle_rad;
                    let sx = cx + dist * sample_angle.cos();
                    let sy = cy + dist * sample_angle.sin();
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
    async fn test_radial_blur_settings() {
        let s = OpImageAdjustmentRadialBlur::settings();
        assert_eq!(s.name, "radial blur");
        assert_eq!(OpImageAdjustmentRadialBlur::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentRadialBlur::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_radial_blur_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("angle".to_string(), Value::Decimal(10.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_radial_blur_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(10.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await;
        assert!(result.is_ok(), "radial_blur 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_radial_blur_uniform_image_unchanged() {
        let uniform = {
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([100u8, 100, 100, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(30.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(4, 4).0;
                assert!((p[0] as i32 - 100).abs() <= 2, "uniform radial blur: expected ~100, got {}", p[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_radial_blur_zero_angle() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageAdjustmentRadialBlur::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
