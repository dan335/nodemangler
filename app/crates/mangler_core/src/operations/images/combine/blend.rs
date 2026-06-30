//! Blend compositing operation.
//!
//! Composites a foreground image onto a background using a configurable blend
//! mode, blend amount, alpha mask, color space, and position offset. Supports
//! all 17 blend modes and all 9 color spaces.

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

/// Operation that blends a foreground image onto a background image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlend {}

impl OpImageCombineBlend {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blend".to_string(),
            description: "Blends an image onto another using a blend mode.".to_string(),
            help: "Walks every background pixel, samples the foreground at the same coordinate minus the position offset, and composites it using the selected BlendMode (Normal, Multiply, Overlay, SoftLight, Difference, and 12 more). Math is performed in the chosen ColorSpace; sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, and YUV are all supported and results are converted back to sRGBA for storage.\n\nThe amount input provides a global opacity; the alpha image (per-pixel, averaged to luminance) multiplies that opacity so you can restrict the blend with a mask. Source alpha channels feed into the blend math. Foreground pixels outside the background's bounds leave the background untouched. Output size is taken from the background; negative positions clamp to zero.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Base image the foreground is composited onto; sets the output size."),
            Input::new("foreground".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Image composited on top of the background using the chosen blend mode."),
            Input::new("amount".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Global opacity applied to the blended foreground; 0 shows only the background."),
            Input::new("alpha".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Optional mask image; its luminance multiplies the blend amount per pixel."),
            Input::new("blend mode".to_string(), Value::BlendMode(crate::color::blend::BlendMode::Over), None, None)
                .with_description("Compositing formula used to combine foreground and background."),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None)
                .with_description("Colour space the blend math runs in (sRGB, Linear, HSL, Lab, etc.)."),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Horizontal offset in pixels from the background's origin to place the foreground."),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Vertical offset in pixels from the background's origin to place the foreground."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
            .with_description("Composited RGBA image sized to match the background.")]
    }

    /// Composites the foreground onto the background using FloatImage directly.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let background_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let foreground_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let blend_mode_converted = convert_input(inputs, 4, ValueType::BlendMode, &mut input_errors);
        let color_space_converted = convert_input(inputs, 5, ValueType::ColorSpace, &mut input_errors);
        let position_x_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);
        let position_y_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data:background, change_id:_} = background_converted.unwrap() else { unreachable!() };
        let Value::Image{data:foreground, change_id:_} = foreground_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::Image{data:alpha, change_id:_} = alpha_converted.unwrap() else { unreachable!() };
        let Value::BlendMode(blend_mode) = blend_mode_converted.unwrap() else { unreachable!() };
        let Value::ColorSpace(color_space) = color_space_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut position_x) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut position_y) = position_y_converted.unwrap() else { unreachable!() };

        position_x = position_x.max(0);
        position_y = position_y.max(0);

        // Helper: get RGBA from any channel count
        let get_rgba = |img: &FloatImage, x: u32, y: u32| -> (f32, f32, f32, f32) {
            let px = img.get_pixel(x, y);
            let ch = img.channels() as usize;
            match ch {
                1 => (px[0], px[0], px[0], 1.0),
                2 => (px[0], px[0], px[0], px[1]),
                3 => (px[0], px[1], px[2], 1.0),
                _ => (px[0], px[1], px[2], px[3]),
            }
        };

        // Output same size as background, 4-channel
        let (bg_w, bg_h) = background.dimensions();
        let mut output = FloatImage::new(bg_w, bg_h, 4);

        for y in 0..bg_h {
            for x in 0..bg_w {
                let (br, bg_val, bb, ba) = get_rgba(&background, x, y);
                let background_color = Color::from_srgb_float(br, bg_val, bb, ba);

                let foreground_x = x as i32 - position_x;
                let foreground_y = y as i32 - position_y;

                if foreground_x >= 0 && foreground_y >= 0
                   && (foreground_x as u32) < foreground.width()
                   && (foreground_y as u32) < foreground.height()
                {
                    let (fr, fg, fb, fa) = get_rgba(&foreground, foreground_x as u32, foreground_y as u32);
                    let mut blend_amount = amount;

                    // Modulate by alpha mask luminance
                    if x < alpha.width() && y < alpha.height() {
                        let apx = alpha.get_pixel(x, y);
                        let ach = alpha.channels() as usize;
                        let alpha_lum = if ach >= 3 { (apx[0] + apx[1] + apx[2]) / 3.0 } else { apx[0] };
                        blend_amount = amount * alpha_lum;
                    }

                    let foreground_color = Color::from_srgb_float(fr, fg, fb, fa);

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
                        crate::color::color_spaces::ColorSpace::Oklab => Color::blend_oklab(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Oklch => Color::blend_oklch(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Hwb => Color::blend_hwb(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Ycbcr => Color::blend_ycbcr(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                        crate::color::color_spaces::ColorSpace::Xyy => Color::blend_xyy(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float(),
                    };

                    output.put_pixel(x, y, &[new_color.0, new_color.1, new_color.2, new_color.3]);
                } else {
                    output.put_pixel(x, y, &[br, bg_val, bb, ba]);
                }
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {value: Value::Image { data: Arc::new(output), change_id:get_id() }}],
        })
    }
}

#[cfg(test)]
#[path = "blend_tests.rs"]
mod tests;
