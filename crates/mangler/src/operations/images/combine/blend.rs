//! Blend compositing operation.
//!
//! Composites a foreground image onto a background using a configurable blend
//! mode, blend amount, alpha mask, color space, and position offset. Supports
//! all 17 blend modes and all 9 color spaces.

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

/// Operation that blends a foreground image onto a background image.
///
/// Supports configurable blend mode (Normal, Multiply, Screen, etc.),
/// blend amount (0.0-1.0), an optional alpha mask image, color space
/// selection for the blending math, and x/y position offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlend {}

impl OpImageCombineBlend {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blend".to_string(),
            description: "Blits an image onto another image.".to_string(),
        }
    }

    /// Creates the input definitions: background, foreground, amount, alpha mask,
    /// blend mode, color space, and x/y position.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("foreground".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("alpha".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(crate::color::blend::BlendMode::Normal), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the output definitions: the composited result image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the operation: composites the foreground onto the background.
    ///
    /// Iterates over every pixel of the background. For each pixel that overlaps
    /// with the positioned foreground, the blend is computed in the selected color
    /// space using the chosen blend mode, modulated by the amount and alpha mask.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let background_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let foreground_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let amount_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::DynamicImage, &mut input_errors);
        let blend_mode_converted = convert_input(inputs, 4, ValueType::BlendMode, &mut input_errors);
        let color_space_converted = convert_input(inputs, 5, ValueType::ColorSpace, &mut input_errors);
        let position_x_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);
        let position_y_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:background, change_id:_} = background_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:foreground, change_id:_} = foreground_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:alpha, change_id:_} = alpha_converted.unwrap() else { unreachable!() };
        let Value::BlendMode(blend_mode) = blend_mode_converted.unwrap() else { unreachable!() };
        let Value::ColorSpace(color_space) = color_space_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut position_x) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut position_y) = position_y_converted.unwrap() else { unreachable!() };

        // run node
        let mut background_image = background.to_rgba32f();
        let foreground_image = foreground.to_rgba32f();
        let alpha_image = alpha.to_rgb32f();
        position_x = position_x.max(0);
        position_y = position_y.max(0);

        for (x, y, pixel) in background_image.enumerate_pixels_mut() {
            let background_color = Color::from_srgb_float(pixel[0], pixel[1], pixel[2], pixel[3]);

            // Compute where this background pixel maps to in the foreground, accounting for offset
            let foreground_x = x as i32 - position_x;
            let foreground_y = y as i32 - position_y;

            if foreground_x >= 0 && foreground_y >= 0 {
                if let Some(foreground_pixel) = foreground_image.get_pixel_checked(foreground_x as u32, foreground_y as u32) {
                    let mut blend_amount = amount;

                    // Modulate the blend amount by the alpha mask's luminance (average of RGB)
                    if let Some(alpha_pixel) = alpha_image.get_pixel_checked(x, y) {
                        blend_amount = amount * ((alpha_pixel[0] as f32 + alpha_pixel[1] as f32 + alpha_pixel[2] as f32) / (1.0 * 3.0));
                    }
    
                    let foreground_color = Color::from_srgb_float(foreground_pixel[0], foreground_pixel[1], foreground_pixel[2], foreground_pixel[3]);
    
                    // Perform the blend in the selected color space and convert back to sRGB
                    let new_color = match color_space {
                        crate::color::color_spaces::ColorSpace::Srgb => Color::blend_srgb(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::RgbLinear => Color::blend_linear(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Hsl => Color::blend_hsl(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Hsv => Color::blend_hsv(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Lch => Color::blend_lch(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Xyz => Color::blend_xyz(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Lab => Color::blend_lab(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Yuv => Color::blend_yuv(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Cmyk => Color::blend_cmyk(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                    };
    
                    *pixel = image::Rgba([new_color.0, new_color.1, new_color.2, new_color.3]);
                }
            }

            
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba32F(background_image)), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::blend::BlendMode;
    use crate::color::color_spaces::ColorSpace;

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
    async fn test_blend_settings() {
        let s = OpImageCombineBlend::settings();
        assert_eq!(s.name, "blend");
        assert_eq!(OpImageCombineBlend::create_inputs().len(), 8);
        assert_eq!(OpImageCombineBlend::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_blend_1x1() {
        let make = |v: u8| {
            let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([v, v, v, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let mut inputs = vec![
            Input::new("background".to_string(), make(100), None, None),
            Input::new("foreground".to_string(), make(200), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("alpha".to_string(), make(255), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Normal), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlend::run(&mut inputs).await;
        assert!(result.is_ok(), "blend 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_blend_amount_zero_is_background() {
        // amount=0 → output should be the background
        let bg = {
            let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([100u8, 100, 100, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let fg = {
            let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200u8, 200, 200, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let alpha = {
            let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 255, 255, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let mut inputs = vec![
            Input::new("background".to_string(), bg, None, None),
            Input::new("foreground".to_string(), fg, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
            Input::new("alpha".to_string(), alpha, None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Normal), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(2, 2).0;
                assert!((p[0] as i32 - 100).abs() <= 2, "amount=0 should be bg (~100), got {}", p[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_all_blend_modes() {
        let modes = [
            BlendMode::Normal, BlendMode::Lerp, BlendMode::Multiply, BlendMode::Screen,
            BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight, BlendMode::ColorDodge,
            BlendMode::ColorBurn, BlendMode::Darken, BlendMode::Lighten, BlendMode::Difference,
            BlendMode::Exclusion, BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::Divide,
            BlendMode::Subtract,
        ];
        for mode in &modes {
            let make = |v: u8| {
                let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([v, v, v, 255]));
                Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
            };
            let mut inputs = vec![
                Input::new("background".to_string(), make(100), None, None),
                Input::new("foreground".to_string(), make(150), None, None),
                Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
                Input::new("alpha".to_string(), make(255), None, None),
                Input::new("blend mode".to_string(), Value::BlendMode(mode.clone()), None, None),
                Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
                Input::new("position x".to_string(), Value::Integer(0), None, None),
                Input::new("position y".to_string(), Value::Integer(0), None, None),
            ];
            let result = OpImageCombineBlend::run(&mut inputs).await;
            assert!(result.is_ok(), "blend mode {:?} failed: {:?}", mode, result.err());
        }
    }

    #[tokio::test]
    async fn test_blend() {
        let mut inputs = vec![
            Input::new("background".to_string(), image_input(4, 4), None, None),
            Input::new("foreground".to_string(), image_input(4, 4), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("alpha".to_string(), image_input(4, 4), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Normal), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
