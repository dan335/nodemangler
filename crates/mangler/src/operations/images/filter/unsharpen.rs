//! Unsharp mask operation for images.
//!
//! Applies an unsharp mask filter using a Gaussian blur subtraction technique.
//! The sigma controls the blur radius and the threshold determines which edges
//! are enhanced (higher threshold = only stronger edges are sharpened).

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

/// Unsharp mask operation that enhances edges by subtracting a blurred version of the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentUnsharpen {}

impl OpImageAdjustmentUnsharpen {
    /// Returns the node metadata (name and description) for the unsharpen operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "unsharpen".to_string(),
            description: "Unsharpens an image.".to_string(),
        }
    }

    /// Creates the input ports: an image, sigma (blur radius), and threshold (edge sensitivity).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None),
            Input::new("threshold".to_string(), Value::Integer(1), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the output port: the unsharp-masked image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the unsharp mask. Clamps sigma to non-negative before applying.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let threshold_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mut sigma) = sigma_converted.unwrap() else { unreachable!() };
        let Value::Integer(threshold) = threshold_converted.unwrap() else { unreachable!() };

        // run node
        sigma = sigma.max(0.0);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(data.unsharpen(sigma, threshold)), change_id:get_id() }},
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
    async fn test_unsharpen() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
            Input::new("threshold".to_string(), Value::Integer(1), None, None),
        ];
        let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_unsharpen_settings() {
        let s = OpImageAdjustmentUnsharpen::settings();
        assert_eq!(s.name, "unsharpen");
        assert_eq!(OpImageAdjustmentUnsharpen::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentUnsharpen::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_unsharpen_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
            Input::new("threshold".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 unsharpen failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_unsharpen_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(16, 8), None, None),
            Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
            Input::new("threshold".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_unsharpen_zero_sigma() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("sigma".to_string(), Value::Decimal(0.0), None, None),
            Input::new("threshold".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await;
        assert!(result.is_ok(), "zero sigma unsharpen failed: {:?}", result.err());
    }
}
