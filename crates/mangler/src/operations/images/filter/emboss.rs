//! Emboss effect operation for images.
//!
//! Creates a 3D-relief emboss effect by computing the difference between
//! pixels sampled along a configurable angle direction. The result is
//! centered around mid-grey (0.5), with raised/lowered areas appearing
//! lighter/darker.

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

/// Emboss operation that creates a 3D-relief effect using directional pixel differences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentEmboss {}

impl OpImageAdjustmentEmboss {
    /// Returns the node metadata (name and description) for the emboss operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "emboss".to_string(),
            description: "Applies an emboss effect.".to_string(),
        }
    }

    /// Creates the input ports: image, intensity, and angle (in degrees) controlling the emboss direction.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
            Input::new("angle".to_string(), Value::Decimal(135.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the embossed image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the emboss effect. Samples forward and backward along the angle direction
    /// and outputs the scaled difference centered at 0.5.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let (width, height) = (buffer.width(), buffer.height());
        let mut output = buffer.clone();
        let intensity = intensity;
        let angle_rad = angle.to_radians();
        // Convert angle to unit direction vector for sampling offsets
        let dx = angle_rad.cos();
        let dy = angle_rad.sin();

        for y in 0..height {
            for x in 0..width {
                let sample = |sx: f32, sy: f32, c: usize| -> f32 {
                    let px = (sx.round() as u32).clamp(0, width - 1);
                    let py = (sy.round() as u32).clamp(0, height - 1);
                    buffer.get_pixel(px, py)[c]
                };

                let fx = x as f32;
                let fy = y as f32;

                let pixel = output.get_pixel_mut(x, y);
                for c in 0..3 {
                    let forward = sample(fx + dx, fy + dy, c);
                    let backward = sample(fx - dx, fy - dy, c);
                    pixel[c] = (0.5 + intensity * (forward - backward)).clamp(0.0, 1.0);
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
    async fn test_emboss_settings() {
        let s = OpImageAdjustmentEmboss::settings();
        assert_eq!(s.name, "emboss");
        assert_eq!(OpImageAdjustmentEmboss::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentEmboss::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_emboss_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
        ];
        let result = OpImageAdjustmentEmboss::run(&mut inputs).await;
        assert!(result.is_ok(), "emboss 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_emboss_uniform_image_is_midgrey() {
        // Emboss of uniform image should produce mid-grey (0.5)
        let uniform = {
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
        ];
        let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba32f().get_pixel(4, 4).0;
                assert!((p[0] - 0.5).abs() < 0.02, "uniform emboss should be ~0.5, got {}", p[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_emboss_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
        ];
        let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
