//! Safe transform operation with wrapping edges for seamless tiling.

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

/// Applies translation, rotation, and scale to an image with wrapping at edges.
///
/// All coordinates wrap around using modular arithmetic, so the output remains
/// seamlessly tileable if the input is tileable. This is especially useful in
/// texture/material workflows where seam-free transforms are required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformSafeTransform {}

impl OpImageTransformSafeTransform {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "safe transform".to_string(),
            description: "Translate, rotate, and scale with wrapping at edges for seamless tiling.".to_string(),
        }
    }

    /// Creates the default inputs: source image, X/Y translation (normalized), rotation (degrees), and scale factor.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None),
            Input::new("scale".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.01, 4.0), step_by: Some(0.01), clamp_to_range: false }), None),
        ]
    }

    /// Creates the default outputs: the transformed image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the safe transform using inverse mapping with wrapping coordinates.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let tx_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let ty_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let rot_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let scale_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tx) = tx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ty) = ty_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rot_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        // Precompute rotation trig values and image center
        let angle_rad = rotation.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        // Prevent division by zero when scale is near zero
        let safe_scale = if scale.abs() < 0.001 { 0.001 } else { scale };

        for y in 0..h {
            for x in 0..w {
                // Inverse transform: from output pixel to source pixel
                // 1. Center
                let px = x as f32 - cx;
                let py = y as f32 - cy;
                // 2. Inverse scale
                let px = px / safe_scale;
                let py = py / safe_scale;
                // 3. Inverse rotate
                let rx = px * cos_a + py * sin_a;
                let ry = -px * sin_a + py * cos_a;
                // 4. Un-center and inverse translate
                let sx = rx + cx - tx * w as f32;
                let sy = ry + cy - ty * h as f32;

                // Wrap coordinates for seamless tiling
                let sx = ((sx % w as f32) + w as f32) % w as f32;
                let sy = ((sy % h as f32) + h as f32) % h as f32;

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

    #[tokio::test]
    async fn test_safe_transform_settings() {
        let s = OpImageTransformSafeTransform::settings();
        assert_eq!(s.name, "safe transform");
        assert_eq!(OpImageTransformSafeTransform::create_inputs().len(), 5);
        assert_eq!(OpImageTransformSafeTransform::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_safe_transform_identity() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 1);
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_safe_transform_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
            Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageTransformSafeTransform::run(&mut inputs).await;
        assert!(result.is_ok(), "safe_transform 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_safe_transform_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 4), None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8, "dimensions should be preserved");
                assert_eq!(data.height(), 4, "dimensions should be preserved");
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_safe_transform_zero_scale_clamped() {
        // scale=0 should be clamped to 0.001 internally and not panic
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
            Input::new("scale".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageTransformSafeTransform::run(&mut inputs).await;
        assert!(result.is_ok(), "safe_transform zero scale should not panic: {:?}", result.err());
    }
}
