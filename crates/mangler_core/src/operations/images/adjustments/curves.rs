//! Tone curve adjustment operation for images.
//!
//! Applies a contrast-like curve centered on a configurable midpoint.
//! Positive strength increases contrast (S-curve), negative strength
//! reduces contrast around the midpoint.

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

/// Tone curve adjustment that applies contrast scaling around a configurable midpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentCurves{}

impl OpImageAdjustmentCurves {
    /// Returns the node metadata (name and description) for the curves operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curves".to_string(),
            description: "Applies a tone curve adjustment.".to_string(),
        }
    }

    /// Creates the input ports: image, strength (-1..1), and midpoint (0..1).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("strength".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the curve-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the curves adjustment. Applies a linear contrast curve centered on the midpoint.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let strength_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let midpoint_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(strength) = strength_converted.unwrap() else { unreachable!() };
        let Value::Decimal(midpoint) = midpoint_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let strength = strength;
        let midpoint = midpoint;
        // Double the strength to get a more perceptually useful contrast range
        let contrast = strength * 2.0;

        for pixel in buffer.pixels_mut() {
            for c in 0..3 {
                let val = pixel[c];
                // Scale deviation from midpoint by the contrast factor
                let adjusted = midpoint + (val - midpoint) * (1.0 + contrast);
                pixel[c] = adjusted.clamp(0.0, 1.0);
            }
            // alpha unchanged
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
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
    async fn test_curves_settings() {
        let s = OpImageAdjustmentCurves::settings();
        assert_eq!(s.name, "curves");
        assert_eq!(OpImageAdjustmentCurves::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentCurves::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_curves_zero_strength_identity() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([128, 128, 128, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("strength".to_string(), Value::Decimal(0.0), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert!((p[0] as i32 - 128).abs() <= 1);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_curves_positive_strength() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("strength".to_string(), Value::Decimal(0.5), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_curves_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("strength".to_string(), Value::Decimal(0.3), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await;
        assert!(result.is_ok(), "curves 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_curves_preserves_dimensions() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("strength".to_string(), Value::Decimal(0.5), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_curves_output_range() {
        // All output pixel values should be in [0, 1]
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("strength".to_string(), Value::Decimal(1.0), None, None),
            Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                for pixel in data.to_rgba32f().pixels() {
                    for c in 0..3 {
                        assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of range: {}", pixel[c]);
                    }
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
