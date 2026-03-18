//! Solid-color image generation operation.
//!
//! Creates an image of a specified width and height where every pixel is
//! filled with the same color. The color is converted to sRGB u8 for storage.

use image::{ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a uniform solid-color image.
///
/// Accepts a color, width, and height, and produces an RGBA image where
/// every pixel is set to the given color. Also passes through the color
/// and dimensions as separate outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputColor {}

impl OpImageInputColor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from color".to_string(),
            description: "Creates an image from a color.".to_string(),
        }
    }

    /// Creates the input definitions: color, width (1-10000), and height (1-10000).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
        ]
    }

    /// Creates the output definitions: the generated image, the color, width, and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("color".to_string(), Value::Color(Color::default()), None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the operation: creates an image buffer filled with the input color.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };

        // run node — clamp dimensions to at least 1
        width = width.max(1);
        height = height.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);
        // Convert the color to 8-bit sRGB once, then fill every pixel
        let rgba = color.to_srgb_u8();

        for x in 0..width {
            for y in 0..height {
                image_buffer.put_pixel(x as u32, y as u32, image::Rgba([rgba.0, rgba.1, rgba.2, rgba.3]));
            }
        }
        
        let dynamic_image = DynamicImage::ImageRgba8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
                OutputResponse { value: Value::Color(color) },
                OutputResponse { value: Value::Integer(width as i32) },
                OutputResponse { value: Value::Integer(height as i32) },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_from_color() {
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImageInputColor::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 4);
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_from_color_settings() {
        let s = OpImageInputColor::settings();
        assert_eq!(s.name, "from color");
        assert_eq!(OpImageInputColor::create_inputs().len(), 3);
        assert_eq!(OpImageInputColor::create_outputs().len(), 4);
    }

    #[tokio::test]
    async fn test_from_color_pixel_values() {
        // All pixels should be the input color
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_u8(255, 0, 128, 200)), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageInputColor::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let rgba = data.to_rgba8();
                // Every pixel should match (within u8 rounding of sRGB)
                for y in 0..4 {
                    for x in 0..4 {
                        let p = rgba.get_pixel(x, y).0;
                        assert_eq!(p[0], 255, "red channel mismatch at ({x},{y})");
                        assert_eq!(p[1], 0, "green channel mismatch at ({x},{y})");
                    }
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_from_color_outputs_color_passthrough() {
        // The second output should be the same color that was input
        let color = Color::from_srgb_float(0.5, 0.25, 0.75, 1.0);
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(color), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageInputColor::run(&mut inputs).await.unwrap();
        match &result.responses[1].value {
            Value::Color(_) => {}
            other => panic!("Expected Color output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_from_color_outputs_width_height() {
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new("width".to_string(), Value::Integer(10), None, None),
            Input::new("height".to_string(), Value::Integer(7), None, None),
        ];
        let result = OpImageInputColor::run(&mut inputs).await.unwrap();
        match &result.responses[2].value {
            Value::Integer(w) => assert_eq!(*w, 10),
            other => panic!("Expected Integer width, got {:?}", other),
        }
        match &result.responses[3].value {
            Value::Integer(h) => assert_eq!(*h, 7),
            other => panic!("Expected Integer height, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_from_color_1x1() {
        let mut inputs = vec![
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("width".to_string(), Value::Integer(1), None, None),
            Input::new("height".to_string(), Value::Integer(1), None, None),
        ];
        let result = OpImageInputColor::run(&mut inputs).await;
        assert!(result.is_ok(), "from_color 1x1 failed: {:?}", result.err());
    }
}
