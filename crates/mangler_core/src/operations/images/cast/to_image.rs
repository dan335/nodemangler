//! Cast-to-image operation for the node graph.
//!
//! Converts a value (bool, integer, decimal, or color) to a 1x1 RGBA image
//! using `try_convert_to`. This provides an explicit cast node for generating
//! images from scalar values.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to a 1x1 image.
///
/// Uses `Value::try_convert_to(ValueType::DynamicImage)` for the conversion.
/// Accepts booleans (black/white), integers (grayscale 0–255), decimals
/// (grayscale 0.0–1.0), and colors (sRGBA pixel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCastToImage {}

impl OpImageCastToImage {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to image".to_string(),
            description: "Converts a value to a 1x1 image.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input (0.0–1.0 grayscale).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), None, None),
        ]
    }

    /// Creates the default output list: a single image output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the cast: converts the input to a DynamicImage via `try_convert_to`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let result = inputs[0].value.try_convert_to(ValueType::DynamicImage);

        match result {
            Ok(image_value) => Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: image_value }],
            }),
            Err(_) => Err(OperationError {
                input_errors: vec![(0, "Unable to convert to image.".to_string())],
                node_error: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;
    use crate::color::Color;

    #[tokio::test]
    async fn test_to_image_settings() {
        let s = OpImageCastToImage::settings();
        assert_eq!(s.name, "to image");
        assert_eq!(OpImageCastToImage::create_inputs().len(), 1);
        assert_eq!(OpImageCastToImage::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_to_image_from_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 1);
                assert_eq!(data.height(), 1);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_image_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(128), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 1);
                assert_eq!(data.height(), 1);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_image_from_bool_true() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let pixel = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(pixel, [255, 255, 255, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_image_from_bool_false() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let pixel = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(pixel, [0, 0, 0, 255]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_image_from_color() {
        let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let mut inputs = vec![Input::new("input".to_string(), Value::Color(color), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let pixel = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(pixel[0], 255); // red
                assert_eq!(pixel[3], 255); // alpha
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_image_from_decimal_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let pixel = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(pixel[0], 0);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_image_from_decimal_one() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let pixel = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(pixel[0], 255);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
