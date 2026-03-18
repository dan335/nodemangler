//! Horizontal flip (mirror left-to-right) operation.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Flips an image horizontally (mirrors left-to-right).
///
/// The operation is performed in-place when possible (single `Arc` reference),
/// otherwise the image data is cloned first. Applying this operation twice
/// restores the original image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformFlipHorizontal {}

impl OpImageTransformFlipHorizontal {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flip horizontal".to_string(),
            description: "Flips an image horizontally.".to_string(),
        }
    }

    /// Creates the default inputs: a single source image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the default outputs: the flipped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the horizontal flip operation in-place.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node
        // Try to take ownership; clone if other references exist
        let mut data_inner = Arc::try_unwrap(data).unwrap_or_else(|a| (*a).clone());
        image::imageops::flip_horizontal_in_place(&mut data_inner);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(data_inner), change_id:get_id() }},
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
    async fn test_flip_horizontal_settings() {
        let s = OpImageTransformFlipHorizontal::settings();
        assert_eq!(s.name, "flip horizontal");
        assert_eq!(OpImageTransformFlipHorizontal::create_inputs().len(), 1);
        assert_eq!(OpImageTransformFlipHorizontal::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_flip_horizontal() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_flip_horizontal_1x1() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
        let result = OpImageTransformFlipHorizontal::run(&mut inputs).await;
        assert!(result.is_ok(), "flip_horizontal 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_flip_horizontal_twice_is_identity() {
        let mut imgbuf = image::RgbaImage::new(4, 4);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x * 60) as u8, (y * 60) as u8, 100, 255]);
        }
        let orig_pixel = imgbuf.get_pixel(1, 2).0;
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        // first flip
        let mut inputs = vec![Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None)];
        let r1 = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
        // second flip
        let mut inputs2 = vec![Input::new("image".to_string(), r1.responses.into_iter().next().unwrap().value, None, None)];
        let r2 = OpImageTransformFlipHorizontal::run(&mut inputs2).await.unwrap();
        match &r2.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(1, 2).0;
                assert_eq!(p, orig_pixel, "double-flip should restore original");
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_flip_horizontal_preserves_dimensions() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
        let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
