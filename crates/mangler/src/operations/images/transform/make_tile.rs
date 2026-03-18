//! Make-tile operation that creates seamlessly tileable images via edge cross-fading.

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

/// Makes an image seamlessly tileable by cross-fading overlapping border regions.
///
/// The blend size parameter (0.01 to 0.5) controls what fraction of the image
/// width/height is used for the cross-fade region. Horizontal edges are blended
/// first, then vertical edges are blended using the already horizontally-blended
/// result to ensure proper corner handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformMakeTile {}

impl OpImageTransformMakeTile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "make tile".to_string(),
            description: "Makes an image tile seamlessly by cross-fading overlapping border regions.".to_string(),
        }
    }

    /// Creates the default inputs: source image and blend size (fraction of image dimensions).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.01, 0.5), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default outputs: the tileable image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the make-tile operation by cross-fading horizontal then vertical edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let blend_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blend_size) = blend_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = src.clone();

        // Compute the pixel-space blend region sizes from the normalized blend fraction
        let blend_size = blend_size.clamp(0.01, 0.5);
        let blend_w = (w as f32 * blend_size) as u32;
        let blend_h = (h as f32 * blend_size) as u32;

        if blend_w == 0 || blend_h == 0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
                ],
            });
        }

        // Cross-fade horizontal edges
        for y in 0..h {
            for bx in 0..blend_w {
                let t = bx as f32 / blend_w as f32; // 0 at left edge, 1 at blend boundary
                let left_x = bx;
                let right_x = w - blend_w + bx;

                let left_pixel = src.get_pixel(left_x, y).0;
                let right_pixel = src.get_pixel(right_x, y).0;

                // Blend: at left edge (t=0), use right pixel; at boundary (t=1), use left pixel
                let blended = blend_pixels(&left_pixel, &right_pixel, t);

                output.put_pixel(left_x, y, image::Rgba(blended));
                output.put_pixel(right_x, y, image::Rgba(blend_pixels(&right_pixel, &left_pixel, t)));
            }
        }

        // Cross-fade vertical edges (use the already-horizontally-blended data)
        let h_blended = output.clone();
        for x in 0..w {
            for by in 0..blend_h {
                let t = by as f32 / blend_h as f32;
                let top_y = by;
                let bottom_y = h - blend_h + by;

                let top_pixel = h_blended.get_pixel(x, top_y).0;
                let bottom_pixel = h_blended.get_pixel(x, bottom_y).0;

                let blended = blend_pixels(&top_pixel, &bottom_pixel, t);
                output.put_pixel(x, top_y, image::Rgba(blended));
                output.put_pixel(x, bottom_y, image::Rgba(blend_pixels(&bottom_pixel, &top_pixel, t)));
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

/// Linearly interpolates between two RGBA pixels by factor `t` (0.0 = fully `b`, 1.0 = fully `a`).
fn blend_pixels(a: &[u8; 4], b: &[u8; 4], t: f32) -> [u8; 4] {
    let mut result = [0u8; 4];
    for i in 0..4 {
        result[i] = (a[i] as f32 * t + b[i] as f32 * (1.0 - t)).clamp(0.0, 255.0) as u8;
    }
    result
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
    async fn test_make_tile_settings() {
        let s = OpImageTransformMakeTile::settings();
        assert_eq!(s.name, "make tile");
        assert_eq!(OpImageTransformMakeTile::create_inputs().len(), 2);
        assert_eq!(OpImageTransformMakeTile::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_make_tile_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 16), None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
        ];
        let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
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
    async fn test_make_tile_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
        ];
        let result = OpImageTransformMakeTile::run(&mut inputs).await;
        assert!(result.is_ok(), "make_tile 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_make_tile_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(32, 16), None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.1), None, None),
        ];
        let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 32);
                assert_eq!(data.height(), 16);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_make_tile_uniform_image_unchanged() {
        // A uniform image tiled should remain the same uniform color
        let uniform = image::RgbaImage::from_pixel(8, 8, image::Rgba([100u8, 150, 200, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(uniform));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
        ];
        let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let rgba = data.to_rgba8();
                // Blending uniform pixels together still gives the same color
                let p = rgba.get_pixel(4, 4).0;
                assert_eq!(p, [100u8, 150, 200, 255], "uniform image should stay uniform after tiling");
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
