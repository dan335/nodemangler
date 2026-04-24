//! Circle shape / gradient image generator.
//!
//! Generates a vertical color gradient between two colors, blended in a
//! configurable color space. Despite the name "circle", this currently produces
//! a vertical gradient strip and outputs width/height alongside the image.
//! Outputs a 4-channel (RGBA) FloatImage.

use std::sync::Arc;
use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that generates a vertical color gradient between two colors.
///
/// Supports blending in multiple color spaces (sRGB, Linear RGB, HSL, HSV,
/// Lab, LCH, XYZ, YUV, CMYK) for perceptually different interpolation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapesCircle {}

impl OpImageShapesCircle {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "circle".to_string(),
            description: "Creates a circle.".to_string(),
            help: "Despite the name, this node currently emits a vertical two-color gradient strip rather than a disc; use the ellipse node for a true circular shape. It fills a 4-channel RGBA FloatImage by blending the color input (top) toward the background (bottom) in the chosen color space using Lerp.\n\nThe padding input is reserved for future circle-layout work and is not used by the current implementation. Colors are quantised to u8 and then re-floated, so expect ~1/255 precision compared with the gradient node.".to_string(),
        }
    }

    /// Creates the default inputs: color, background, width, height, padding, color space, and blend mode.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Starting color at the top of the vertical gradient."),
            Input::new("background".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None)
                .with_description("Ending color at the bottom of the vertical gradient."),
            Input::new("width".to_string(), Value::Decimal(512.0), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Decimal(512.0), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("padding".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Padding value reserved for circle layout (currently unused)."),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None)
                .with_description("Color space used to interpolate between the two colors."),
            Input::new("blend mode".to_string(), Value::BlendMode(crate::color::blend::BlendMode::Lerp), None, None)
                .with_description("Blend mode used when mixing the two gradient colors."),
        ]
    }

    /// Creates the default outputs: the gradient image, width, and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("RGBA gradient image blending the two colors from top to bottom."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Final width of the generated image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Final height of the generated image in pixels."),
        ]
    }

    /// Generates a vertical color gradient image blended in the selected color space.
    ///
    /// The output is a 4-channel (RGBA) FloatImage with values in [0.0, 1.0].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Color(a)) = inputs[0].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Color(b)) = inputs[1].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let Ok(Value::Integer(mut width)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[3].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let Ok(Value::ColorSpace(color_space)) = inputs[4].value.try_convert_to(ValueType::ColorSpace) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        width = width.max(1);
        height = height.max(1);

        // 4-channel RGBA output
        let mut image = FloatImage::new(width as u32, height as u32, 4);

        let blend_mode = crate::color::blend::BlendMode::Lerp;

        /// Helper to write a blended color row into the FloatImage.
        fn write_row(image: &mut FloatImage, y: i32, width: i32, blended: (u8, u8, u8, u8)) {
            // Convert u8 sRGB to f32 [0.0, 1.0]
            let rf = blended.0 as f32 / 255.0;
            let gf = blended.1 as f32 / 255.0;
            let bf = blended.2 as f32 / 255.0;
            let af = blended.3 as f32 / 255.0;
            for x in 0..width {
                image.put_pixel(x as u32, y as u32, &[rf, gf, bf, af]);
            }
        }

        // Blend the two colors row-by-row using the selected color space
        match color_space {
            ColorSpace::Srgb => {
                for y in 0..height {
                    let blended = Color::blend_srgb(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::RgbLinear => {
                for y in 0..height {
                    let blended = Color::blend_linear(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Hsl => {
                for y in 0..height {
                    let blended = Color::blend_hsl(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Hsv => {
                for y in 0..height {
                    let blended = Color::blend_hsv(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Lch => {
                for y in 0..height {
                    let blended = Color::blend_lch(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Xyz => {
                for y in 0..height {
                    let blended = Color::blend_xyz(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Lab => {
                for y in 0..height {
                    let blended = Color::blend_lab(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Yuv => {
                for y in 0..height {
                    let blended = Color::blend_yuv(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
            ColorSpace::Cmyk => {
                for y in 0..height {
                    let blended = Color::blend_cmyk(a, b, &blend_mode, y as f32 / height as f32).to_srgb_u8();
                    write_row(&mut image, y, width, blended);
                }
            },
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
                OutputResponse { value: Value::Integer(width as i32) },
                OutputResponse { value: Value::Integer(height as i32) },
            ],
        })
    }
}


#[cfg(test)]
#[path = "circle_tests.rs"]
mod tests;
