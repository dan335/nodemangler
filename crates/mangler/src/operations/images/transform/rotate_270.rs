//! 270-degree clockwise (90-degree counter-clockwise) rotation operation.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Rotates an image 270 degrees clockwise (equivalent to 90 degrees counter-clockwise).
///
/// The output dimensions are swapped: width becomes height and vice versa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformRotate270 {}

impl OpImageTransformRotate270 {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate 270".to_string(),
            description: "Rotates an image 270 degrees.".to_string(),
        }
    }

    /// Creates the default inputs: a single source image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
        ]
    }

    /// Creates the default outputs: the rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the 270-degree clockwise rotation.
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
        let im = data.rotate270();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(im), change_id:get_id() }},
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
    async fn test_rotate_270_settings() {
        let s = OpImageTransformRotate270::settings();
        assert_eq!(s.name, "rotate 270");
        assert_eq!(OpImageTransformRotate270::create_inputs().len(), 1);
        assert_eq!(OpImageTransformRotate270::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_rotate_270() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
        let result = OpImageTransformRotate270::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rotate_270_1x1() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
        let result = OpImageTransformRotate270::run(&mut inputs).await;
        assert!(result.is_ok(), "rotate_270 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_rotate_270_swaps_dimensions() {
        let mut inputs = vec![Input::new("image".to_string(), image_input(8, 4), None, None)];
        let result = OpImageTransformRotate270::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 4, "width should become height after 270");
                assert_eq!(data.height(), 8, "height should become width after 270");
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
