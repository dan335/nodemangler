//! Mirror operation that reflects image content across configurable axes.

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

/// Mirrors an image across the X axis, Y axis, or both, with configurable split offsets.
///
/// The offset parameters (0.0 to 1.0) control where the mirror axis sits within the image.
/// At 0.5, the mirror axis is at the center. Pixels on one side of the axis are reflected
/// onto the other side, creating a symmetric result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformMirror {}

impl OpImageTransformMirror {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mirror".to_string(),
            description: "Mirrors an image across X, Y, or both axes with configurable offset.".to_string(),
        }
    }

    /// Creates the default inputs: source image, mirror X/Y toggles, and X/Y offset positions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default outputs: the mirrored image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the mirror operation by reflecting pixels across the configured axes.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let mirror_x_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);
        let mirror_y_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);
        let offset_x_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let offset_y_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Bool(mirror_x) = mirror_x_converted.unwrap() else { unreachable!() };
        let Value::Bool(mirror_y) = mirror_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_x) = offset_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_y) = offset_y_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        // Convert normalized offsets to pixel positions for the mirror axes
        let split_x = (w as f32 * offset_x.clamp(0.0, 1.0)) as u32;
        let split_y = (h as f32 * offset_y.clamp(0.0, 1.0)) as u32;

        for y in 0..h {
            for x in 0..w {
                let sx = if mirror_x && x >= split_x {
                    // Reflect: compute distance past the split and map back symmetrically
                    let dist = x - split_x;
                    if split_x as i32 - dist as i32 > 0 {
                        split_x - dist - 1
                    } else {
                        0
                    }
                } else {
                    x
                };

                let sy = if mirror_y && y >= split_y {
                    let dist = y - split_y;
                    if split_y as i32 - dist as i32 > 0 {
                        split_y - dist - 1
                    } else {
                        0
                    }
                } else {
                    y
                };

                let sx = sx.min(w - 1);
                let sy = sy.min(h - 1);
                output.put_pixel(x, y, *src.get_pixel(sx, sy));
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
    async fn test_mirror_settings() {
        let s = OpImageTransformMirror::settings();
        assert_eq!(s.name, "mirror");
        assert_eq!(OpImageTransformMirror::create_inputs().len(), 5);
        assert_eq!(OpImageTransformMirror::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_mirror_x_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
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
    async fn test_mirror_x_symmetry() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let rgba = data.to_rgba8();
                let left = rgba.get_pixel(3, 0).0;
                let right = rgba.get_pixel(4, 0).0;
                assert_eq!(left, right);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mirror_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(true), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await;
        assert!(result.is_ok(), "mirror 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_mirror_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 4), None, None),
            Input::new("mirror x".to_string(), Value::Bool(false), None, None),
            Input::new("mirror y".to_string(), Value::Bool(true), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 4);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mirror_no_mirror_is_passthrough() {
        // With both mirrors off, the output should match the input
        let uniform = image::RgbaImage::from_pixel(8, 8, image::Rgba([77u8, 88, 99, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(uniform));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("mirror x".to_string(), Value::Bool(false), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p, [77u8, 88, 99, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
