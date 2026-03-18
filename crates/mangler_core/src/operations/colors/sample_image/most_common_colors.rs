//! Most common colors sampling operation.
//!
//! Analyzes an image to find the most frequently occurring colors by
//! quantizing each pixel's HSL representation and counting occurrences.
//! Returns the top 5 most common colors.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::collections::HashMap;

/// Operation that extracts the top 5 most common colors from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorSampleMostCommonColors {}

impl OpColorSampleMostCommonColors {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "most common colors".to_string(),
            description: "Finds the most common colors in an image.".to_string(),
        }
    }

    /// Creates the input definitions: an image and quantization precision for hue, saturation, and lightness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage{data:crate::operations::default_image(), change_id:crate::get_id()}, None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
        ]
    }

    /// Creates 5 color output slots, one for each of the top most common colors.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("1".to_string(), Value::Color(Color::default()), None),
            Output::new("2".to_string(), Value::Color(Color::default()), None),
            Output::new("3".to_string(), Value::Color(Color::default()), None),
            Output::new("4".to_string(), Value::Color(Color::default()), None),
            Output::new("5".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the operation, scanning all pixels and returning the 5 most common quantized colors.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let hue_precision_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let saturation_precision_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let lightness_precision_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:image, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(hue_precision) = hue_precision_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation_precision) = saturation_precision_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lightness_precision) = lightness_precision_converted.unwrap() else { unreachable!() };

        // Quantize each pixel's HSL values into buckets and count occurrences.
        // Higher precision values produce more buckets (finer color distinction).
        let mut color_counts: HashMap<[i32; 3], u32> = HashMap::new();

        for rgb in image::Rgb32FImage::pixels(&image.to_rgb32f()) {
            let color = Color::from_srgb_float(rgb[0], rgb[1], rgb[2], 1.0);
            let hsl = color.to_hsl();
            // Round each channel to its quantized bucket index
            let h = ((hsl.0 / 360.0) * hue_precision).round() as i32;
            let s = (hsl.1 * saturation_precision).round() as i32;
            let l = (hsl.2 * lightness_precision).round() as i32;
            *color_counts.entry([h, s, l]).or_insert(0) += 1;
        }

        // Sort buckets by pixel count (most frequent first)
        let mut sorted_colors: Vec<(&[i32; 3], &u32)> = color_counts.iter().collect();
        sorted_colors.sort_by(|a, b| b.1.cmp(a.1));

        let mut responses: Vec<OutputResponse> = Vec::new();

        // Convert the top 5 quantized HSL buckets back to colors
        for (hsl, _count) in sorted_colors.iter().take(5) {
            let h = ((hsl[0] as f32) / hue_precision) * 360.0;
            let s = (hsl[1] as f32) / saturation_precision;
            let l = (hsl[2] as f32) / lightness_precision;
            responses.push(OutputResponse {
                value: Value::Color(Color::from_hsl(h, s, l, 1.0)),
            });
        }

        // Pad with default colors if fewer than 5 distinct buckets exist
        while responses.len() < 5 {
            responses.push(OutputResponse {
                value: Value::Color(Color::default()),
            });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses,
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

    fn test_image(w: u32, h: u32) -> Value {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            *pixel = image::Rgba([r, g, 128, 255]);
        }
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }
    }

    #[tokio::test]
    async fn test_most_common_colors() {
        let mut inputs = vec![
            Input::new("image".to_string(), test_image(4, 4), None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
        assert!(result.responses.len() <= 5);
        for resp in &result.responses {
            match &resp.value {
                Value::Color(_) => {}
                other => panic!("Expected Color, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_most_common_colors_settings() {
        let s = OpColorSampleMostCommonColors::settings();
        assert_eq!(s.name, "most common colors");
        assert_eq!(OpColorSampleMostCommonColors::create_inputs().len(), 4);
        assert_eq!(OpColorSampleMostCommonColors::create_outputs().len(), 5);
    }

    #[tokio::test]
    async fn test_most_common_colors_always_five_responses() {
        let mut inputs = vec![
            Input::new("image".to_string(), test_image(4, 4), None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 5, "should always return exactly 5 colors");
    }

    #[tokio::test]
    async fn test_most_common_colors_uniform_image() {
        // Uniform image: all pixels the same color — top result should be approximately that color
        let mut imgbuf = image::RgbaImage::new(4, 4);
        for pixel in imgbuf.pixels_mut() {
            *pixel = image::Rgba([255u8, 0, 0, 255]);
        }
        let img = Value::DynamicImage {
            data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
            change_id: get_id(),
        };
        let mut inputs = vec![
            Input::new("image".to_string(), img, None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 5);
        // At least the first should be a valid Color
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_most_common_colors_1x1_image() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([128u8, 64, 32, 255]));
        let img = Value::DynamicImage {
            data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
            change_id: get_id(),
        };
        let mut inputs = vec![
            Input::new("image".to_string(), img, None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(5.0), None, None),
            Input::new("saturation quantization".to_string(), Value::Decimal(5.0), None, None),
            Input::new("lightness quantization".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpColorSampleMostCommonColors::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 image most_common_colors failed: {:?}", result.err());
    }
}
