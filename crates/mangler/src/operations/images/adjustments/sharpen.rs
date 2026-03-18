//! Convolution-based sharpening operation for images.
//!
//! Applies a 3x3 sharpening kernel where the center weight is boosted and
//! edge weights are negative, enhancing local contrast at edges.

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
use image::DynamicImage;

/// Convolution-based sharpening operation using a 3x3 edge-enhancement kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSharpen {}

impl OpImageAdjustmentSharpen {
    /// Returns the node metadata (name and description) for the sharpen operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sharpen".to_string(),
            description: "Sharpens an image using convolution.".to_string(),
        }
    }

    /// Creates the input ports: an image and an intensity controlling sharpening strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the sharpened image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the sharpening convolution. Uses edge-clamped sampling for border pixels.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let (width, height) = (buffer.width(), buffer.height());
        let mut output = buffer.clone();
        let intensity = intensity as f32;

        // Sharpen kernel: center = 1 + 4*intensity, edges = -intensity, corners = 0
        let center = 1.0 + 4.0 * intensity;
        let edge = -intensity;

        for y in 0..height {
            for x in 0..width {
                let sample = |px: u32, py: u32| -> [f32; 3] {
                    let p = buffer.get_pixel(px.clamp(0, width - 1), py.clamp(0, height - 1));
                    [p[0], p[1], p[2]]
                };

                let x0 = if x > 0 { x - 1 } else { 0 };
                let x2 = if x + 1 < width { x + 1 } else { width - 1 };
                let y0 = if y > 0 { y - 1 } else { 0 };
                let y2 = if y + 1 < height { y + 1 } else { height - 1 };

                let c_val = sample(x, y);
                let top = sample(x, y0);
                let bottom = sample(x, y2);
                let left = sample(x0, y);
                let right = sample(x2, y);

                let pixel = output.get_pixel_mut(x, y);
                for c in 0..3 {
                    let val = center * c_val[c]
                        + edge * top[c]
                        + edge * bottom[c]
                        + edge * left[c]
                        + edge * right[c];
                    pixel[c] = val.clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
        }

        let adjusted = DynamicImage::ImageRgba32F(output);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(adjusted), change_id: get_id() } },
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
    async fn test_sharpen_settings() {
        let s = OpImageAdjustmentSharpen::settings();
        assert_eq!(s.name, "sharpen");
        assert_eq!(OpImageAdjustmentSharpen::create_inputs().len(), 2);
        assert_eq!(OpImageAdjustmentSharpen::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_sharpen_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentSharpen::run(&mut inputs).await;
        assert!(result.is_ok(), "sharpen 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_sharpen_zero_intensity_is_identity() {
        // Zero intensity → kernel center=1, edges=0, so output = original
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert!((p[0] as i32 - 200).abs() <= 1, "zero-sharpen R mismatch: {}", p[0]);
                assert!((p[1] as i32 - 100).abs() <= 1, "zero-sharpen G mismatch: {}", p[1]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sharpen_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
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
    async fn test_sharpen_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
