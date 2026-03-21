//! Gradient image generation operation.
//!
//! Creates a vertical linear gradient image by blending two colors from top
//! to bottom. The blending is performed in a user-selectable color space
//! (sRGB, Linear RGB, HSL, HSV, Lab, LCH, XYZ, YUV, or CMYK) using Lerp
//! interpolation. The output is a 4-channel `FloatImage` with sRGB float values.

use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::float_image::FloatImage;
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
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None),
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

        // Create a 4-channel FloatImage to hold the gradient output
        let mut float_img = FloatImage::new(width as u32, height as u32, 4);

        // Use Lerp blend mode for smooth linear interpolation between colors
        let blend_mode = crate::color::blend::BlendMode::Lerp;

        // Select the blend function for the chosen color space
        let blend_fn: fn(Color, Color, &crate::color::blend::BlendMode, f32) -> Color = match color_space {
            ColorSpace::Srgb      => Color::blend_srgb,
            ColorSpace::RgbLinear => Color::blend_linear,
            ColorSpace::Hsl       => Color::blend_hsl,
            ColorSpace::Hsv       => Color::blend_hsv,
            ColorSpace::Lch       => Color::blend_lch,
            ColorSpace::Xyz       => Color::blend_xyz,
            ColorSpace::Lab       => Color::blend_lab,
            ColorSpace::Yuv       => Color::blend_yuv,
            ColorSpace::Cmyk      => Color::blend_cmyk,
        };

        // Blend per-row in the selected color space, storing sRGB floats directly
        for y in 0..height {
            let blended = blend_fn(a, b, &blend_mode, y as f32 / height as f32);
            let srgb = blended.to_srgb_float();
            let pixel = [srgb.0, srgb.1, srgb.2, srgb.3];
            for x in 0..width {
                float_img.put_pixel(x as u32, y as u32, &pixel);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_img), change_id: get_id() } },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "gradient_tests.rs"]
mod tests;
