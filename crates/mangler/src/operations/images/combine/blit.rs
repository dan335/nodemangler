//! Blit (pixel-copy overlay) compositing operation.
//!
//! Overlays a foreground image onto a background image at a specified x/y
//! position using alpha-aware pixel copying via `image::imageops::overlay`.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that blits (overlays) a foreground image onto a background.
///
/// Unlike the blend operation, this performs a simple alpha-composited overlay
/// without blend modes, amount controls, or color space selection. It delegates
/// to `image::imageops::overlay` for the actual pixel compositing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlit {}

impl OpImageCombineBlit {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blit".to_string(),
            description: "Blits an image onto another image.".to_string(),
        }
    }

    /// Creates the input definitions: background image, foreground image, and x/y position.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("foreground".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the output definitions: the composited result image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the operation: overlays the foreground onto the background at the given position.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let background_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let foreground_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let position_x_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let position_y_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:background_arc, change_id:_} = background_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:foreground, change_id:_} = foreground_converted.unwrap() else { unreachable!() };
        let Value::Integer(x) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(y) = position_y_converted.unwrap() else { unreachable!() };

        // run node — try to take ownership of the background to avoid cloning if possible
        let mut background = Arc::try_unwrap(background_arc).unwrap_or_else(|a| (*a).clone());
        image::imageops::overlay(&mut background, &*foreground, x as i64, y as i64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(background), change_id:get_id() }},
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
    async fn test_blit_settings() {
        let s = OpImageCombineBlit::settings();
        assert_eq!(s.name, "blit");
        assert_eq!(OpImageCombineBlit::create_inputs().len(), 4);
        assert_eq!(OpImageCombineBlit::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_blit_1x1() {
        let bg = {
            let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([50u8, 50, 50, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let fg = {
            let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([200u8, 200, 200, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let mut inputs = vec![
            Input::new("background".to_string(), bg, None, None),
            Input::new("foreground".to_string(), fg, None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlit::run(&mut inputs).await;
        assert!(result.is_ok(), "blit 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_blit_out_of_bounds_position() {
        // Blit a foreground placed completely outside the background - should not crash
        let mut inputs = vec![
            Input::new("background".to_string(), image_input(4, 4), None, None),
            Input::new("foreground".to_string(), image_input(4, 4), None, None),
            Input::new("position x".to_string(), Value::Integer(100), None, None),
            Input::new("position y".to_string(), Value::Integer(100), None, None),
        ];
        let result = OpImageCombineBlit::run(&mut inputs).await;
        assert!(result.is_ok(), "blit out-of-bounds failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_blit_preserves_background_dimensions() {
        let mut inputs = vec![
            Input::new("background".to_string(), image_input(8, 8), None, None),
            Input::new("foreground".to_string(), image_input(4, 4), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blit() {
        let mut inputs = vec![
            Input::new("background".to_string(), image_input(8, 8), None, None),
            Input::new("foreground".to_string(), image_input(4, 4), None, None),
            Input::new("position x".to_string(), Value::Integer(2), None, None),
            Input::new("position y".to_string(), Value::Integer(2), None, None),
        ];
        let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
