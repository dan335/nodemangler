//! Gradient image generation operation.
//!
//! Creates a vertical linear gradient image by blending two colors from top
//! to bottom. The blending is performed in a user-selectable color space
//! (sRGB, Linear RGB, HSL, HSV, Lab, LCH, XYZ, YUV, or CMYK) using Lerp
//! interpolation.

use image::{ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a vertical gradient image between two colors.
///
/// The gradient runs from color `a` (top) to color `b` (bottom), interpolated
/// in the selected color space. Each row is computed once and replicated across
/// all columns for efficiency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputGradient {}

impl OpImageInputGradient {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from gradient".to_string(),
            description: "Creates an image from a gradient.".to_string(),
        }
    }

    /// Creates the input definitions: two colors (a and b), width, height, and color space.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_u8(255, 255, 255, 255)), None, None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None),
        ]
    }

    /// Creates the output definitions: the gradient image, width, and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the operation: generates a vertical gradient by blending colors row by row.
    ///
    /// The blend factor for each row is `y / height`, so the top row is fully color `a`
    /// and the bottom row is fully color `b`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let width_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let color_space_converted = convert_input(inputs, 4, ValueType::ColorSpace, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::ColorSpace(color_space) = color_space_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        // Use Lerp blend mode for smooth linear interpolation between colors
        let blend_mode = crate::color::blend::BlendMode::Lerp;

        // Blend per-row in the selected color space, converting each result to sRGB u8
        match color_space {
            ColorSpace::Srgb => {
                for y in 0..height {
                    let blended = Color::blend_srgb(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::RgbLinear => {
                for y in 0..height {
                    let blended = Color::blend_linear(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Hsl => {
                for y in 0..height {
                    let blended = Color::blend_hsl(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Hsv => {
                for y in 0..height {
                    let blended = Color::blend_hsv(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Lch => {
                for y in 0..height {
                    let blended = Color::blend_lch(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Xyz => {
                for y in 0..height {
                    let blended = Color::blend_xyz(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Lab => {
                for y in 0..height {
                    let blended = Color::blend_lab(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Yuv => {
                for y in 0..height {
                    let blended = Color::blend_yuv(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
            ColorSpace::Cmyk => {
                for y in 0..height {
                    let blended = Color::blend_cmyk(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    for x in 0..width {
                        image_buffer.put_pixel(x as u32, y as u32, image::Rgba([blended.0, blended.1, blended.2, blended.3]));
                    }
                }
            },
        }

        
        
        let dynamic_image = DynamicImage::ImageRgba8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "gradient_tests.rs"]
mod tests;
