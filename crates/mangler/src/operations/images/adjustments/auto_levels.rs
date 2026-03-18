//! Automatic levels adjustment operation for images.
//!
//! Analyzes the image histogram to find the actual luminance range, then
//! remaps pixel values to fill the full [0, 1] range. Configurable clip
//! percentages allow ignoring outlier pixels at both ends.

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

/// Automatic levels adjustment that stretches the histogram to fill the full tonal range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentAutoLevels{}

impl OpImageAdjustmentAutoLevels {
    /// Returns the node metadata (name and description) for the auto levels operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "auto levels".to_string(),
            description: "Automatically adjusts white and black points.".to_string(),
        }
    }

    /// Creates the input ports: image and clip percentages for black and white ends of the histogram.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("clip black".to_string(), Value::Decimal(0.005), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.001), clamp_to_range: true }), None),
            Input::new("clip white".to_string(), Value::Decimal(0.005), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.001), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the auto-levels-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the auto levels adjustment. Builds a 256-bin luminance histogram,
    /// finds clip-adjusted black and white points, then linearly remaps all channels.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let clip_black_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let clip_white_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clip_black) = clip_black_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clip_white) = clip_white_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let clip_black = clip_black as f32;
        let clip_white = clip_white as f32;

        // build 256-bin histogram of luminance values
        let mut histogram = [0u32; 256];
        let total_pixels = buffer.pixels().len() as f32;
        for pixel in buffer.pixels() {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            let bin = (lum * 255.0).clamp(0.0, 255.0) as usize;
            histogram[bin] += 1;
        }

        // find black point: luminance where clip_black fraction of pixels are below
        let black_threshold = (clip_black * total_pixels) as u32;
        let mut cumulative = 0u32;
        let mut black_point = 0.0_f32;
        for (i, &count) in histogram.iter().enumerate() {
            cumulative += count;
            if cumulative >= black_threshold {
                black_point = i as f32 / 255.0;
                break;
            }
        }

        // find white point: luminance where clip_white fraction of pixels are above
        let white_threshold = (clip_white * total_pixels) as u32;
        cumulative = 0;
        let mut white_point = 1.0_f32;
        for (i, &count) in histogram.iter().enumerate().rev() {
            cumulative += count;
            if cumulative >= white_threshold {
                white_point = i as f32 / 255.0;
                break;
            }
        }

        // remap if valid range
        if white_point > black_point {
            let range = white_point - black_point;
            for pixel in buffer.pixels_mut() {
                for c in 0..3 {
                    let val = pixel[c];
                    pixel[c] = ((val - black_point) / range).clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
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
    async fn test_auto_levels_settings() {
        let s = OpImageAdjustmentAutoLevels::settings();
        assert_eq!(s.name, "auto levels");
        assert_eq!(OpImageAdjustmentAutoLevels::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentAutoLevels::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_auto_levels_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([128u8, 64, 32, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("clip black".to_string(), Value::Decimal(0.0), None, None),
            Input::new("clip white".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await;
        assert!(result.is_ok(), "auto_levels 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_auto_levels_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
            Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
        ];
        let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
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

    #[tokio::test]
    async fn test_auto_levels_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
            Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
        ];
        let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
