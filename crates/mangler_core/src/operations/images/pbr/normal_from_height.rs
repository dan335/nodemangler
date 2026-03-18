//! Normal map generation from a height map.
//!
//! Computes a tangent-space normal map from a grayscale height map using the
//! Sobel operator for gradient estimation. The resulting normals are encoded
//! in the standard `[0, 1]` range where `(0.5, 0.5, 1.0)` represents a flat surface.

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

/// Operation that generates a normal map from a grayscale height map.
///
/// Uses a 3x3 Sobel operator to compute horizontal and vertical gradients,
/// then derives the surface normal from those gradients scaled by the
/// intensity parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrNormalFromHeight{}

impl OpImagePbrNormalFromHeight {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "normal from height".to_string(),
            description: "Generates a normal map from a grayscale height map.".to_string(),
        }
    }

    /// Creates the default inputs: the height map image and intensity multiplier.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 20.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single RGBA32F normal map image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Generates a normal map from the input height map using the Sobel operator.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node
        let rgba = data.to_rgba32f();
        let width = rgba.width() as i32;
        let height = rgba.height() as i32;
        let intensity = intensity;

        // Compute luminance (Rec. 709) of a pixel, clamping coords to image bounds
        let luminance = |x: i32, y: i32| -> f32 {
            let cx = x.clamp(0, width - 1) as u32;
            let cy = y.clamp(0, height - 1) as u32;
            let p = rgba.get_pixel(cx, cy);
            0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
        };

        let mut buffer = image::ImageBuffer::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                let tl = luminance(x - 1, y - 1);
                let top = luminance(x, y - 1);
                let tr = luminance(x + 1, y - 1);
                let left = luminance(x - 1, y);
                let right = luminance(x + 1, y);
                let bl = luminance(x - 1, y + 1);
                let bottom = luminance(x, y + 1);
                let br = luminance(x + 1, y + 1);

                // Sobel operator
                let dx = (tr + 2.0 * right + br) - (tl + 2.0 * left + bl);
                let dy = (bl + 2.0 * bottom + br) - (tl + 2.0 * top + tr);

                // Scale by intensity
                let dx = dx * intensity;
                let dy = dy * intensity;

                // Compute and normalize normal vector
                let nx = -dx;
                let ny = -dy;
                let nz = 1.0_f32;
                let len = (nx * nx + ny * ny + nz * nz).sqrt();
                let nx = nx / len;
                let ny = ny / len;
                let nz = nz / len;

                // Map from [-1,1] to [0,1]
                let r = nx * 0.5 + 0.5;
                let g = ny * 0.5 + 0.5;
                let b = nz * 0.5 + 0.5;

                buffer.put_pixel(x as u32, y as u32, image::Rgba([r, g, b, 1.0]));
            }
        }

        let result = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(result), change_id:get_id() }},
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
    use image::{DynamicImage, RgbaImage};
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let r = ((x as f32 / w as f32) * 255.0) as u8;
                let g = ((y as f32 / h as f32) * 255.0) as u8;
                img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
            }
        }
        Arc::new(DynamicImage::ImageRgba8(img))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }


    #[tokio::test]
    async fn test_opimagepbrnormalfromheight_settings() {
        let s = OpImagePbrNormalFromHeight::settings();
        assert_eq!(s.name, "normal from height");
        assert_eq!(OpImagePbrNormalFromHeight::create_inputs().len(), 2);
        assert_eq!(OpImagePbrNormalFromHeight::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagepbrnormalfromheight_run() {
        let mut inputs = vec![
            Input::new("img".to_string(), image_input(16, 16), None, None),
            Input::new("i1".to_string(), Value::Decimal(1.0), None, None)
        ];
        let result = OpImagePbrNormalFromHeight::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrnormalfromheight_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrNormalFromHeight::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 normal_from_height failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_opimagepbrnormalfromheight_uniform_flat() {
        // Flat uniform height map -> all normals should point straight up (B ~= 1.0 mapped to ~1.0)
        let flat = Arc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]))
        ));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: flat, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                let px = buf.get_pixel(4, 4);
                // R and G should be ~0.5 (zero normal x/y), B should be ~1.0 (pointing up)
                assert!((px[0] - 0.5).abs() < 0.05, "flat R should be ~0.5, got {}", px[0]);
                assert!((px[1] - 0.5).abs() < 0.05, "flat G should be ~0.5, got {}", px[1]);
                assert!(px[2] > 0.9, "flat B (up direction) should be >0.9, got {}", px[2]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrnormalfromheight_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrnormalfromheight_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                for px in buf.pixels() {
                    assert!(px[0] >= 0.0 && px[0] <= 1.0, "R out of range: {}", px[0]);
                    assert!(px[1] >= 0.0 && px[1] <= 1.0, "G out of range: {}", px[1]);
                    assert!(px[2] >= 0.0 && px[2] <= 1.0, "B out of range: {}", px[2]);
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

}
