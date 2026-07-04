//! Blend compositing operation.
//!
//! Composites a foreground image onto a background using a configurable blend
//! mode, blend amount, alpha mask, color space, and position offset. Supports
//! all 17 blend modes and all 14 color spaces.

use crate::color::Color;
use crate::color::blend::{BlendMode, lerp, per_channel_fn};
use crate::color::color_spaces::ColorSpace;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
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
            help: "Walks every background pixel, samples the foreground at the same coordinate minus the position offset, and composites it using the selected BlendMode (Over, Lerp, Multiply, Overlay, SoftLight, and 12 more). Math is performed in the chosen ColorSpace; all 14 spaces (sRGB, Linear RGB, HSL, HSV, HWB, Lab, LCH, Oklab, Oklch, CMYK, XYZ, xyY, YCbCr, YUV) are supported and results are converted back to sRGBA for storage.\n\nThe amount input provides a global opacity; the alpha image (per-pixel, RGB channels averaged) multiplies that opacity so you can restrict the blend with a mask. Source alpha channels feed into the blend math. Foreground pixels outside the background's bounds leave the background untouched. Output size is taken from the background; positions may be negative to shift the foreground past the top-left edge.".to_string(),
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
                .with_description("Optional mask image; its per-pixel RGB average multiplies the blend amount."),
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
        let Value::Integer(position_x) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(position_y) = position_y_converted.unwrap() else { unreachable!() };

        // Output same size as background, 4-channel
        let (bg_w, bg_h) = background.dimensions();
        let bg_ch = background.channels() as usize;
        let (fg_w, fg_h) = foreground.dimensions();
        let fg_ch = foreground.channels() as usize;
        let (al_w, al_h) = alpha.dimensions();
        let al_ch = alpha.channels() as usize;

        // Select the blend function for the chosen color space once, outside the pixel loop
        let blend_fn: fn(Color, Color, &BlendMode, f32) -> Color = match color_space {
            ColorSpace::Srgb      => Color::blend_srgb,
            ColorSpace::RgbLinear => Color::blend_linear,
            ColorSpace::Hsl       => Color::blend_hsl,
            ColorSpace::Hsv       => Color::blend_hsv,
            ColorSpace::Lch       => Color::blend_lch,
            ColorSpace::Xyz       => Color::blend_xyz,
            ColorSpace::Lab       => Color::blend_lab,
            ColorSpace::Yuv       => Color::blend_yuv,
            ColorSpace::Cmyk      => Color::blend_cmyk,
            ColorSpace::Oklab     => Color::blend_oklab,
            ColorSpace::Oklch     => Color::blend_oklch,
            ColorSpace::Hwb       => Color::blend_hwb,
            ColorSpace::Ycbcr     => Color::blend_ycbcr,
            ColorSpace::Xyy       => Color::blend_xyy,
        };

        // sRGB is the identity space here: from_srgb_float stores the raw channels
        // and to_srgb_float only clamps to [0, 1], so we can blend raw f32 channels
        // directly without constructing Color round-trips.
        let srgb_fast: Option<SrgbFastBlend> = if matches!(color_space, ColorSpace::Srgb) {
            Some(match blend_mode {
                BlendMode::Over => SrgbFastBlend::Over,
                BlendMode::Lerp => SrgbFastBlend::Lerp,
                _ => SrgbFastBlend::Ch(per_channel_fn(&blend_mode)),
            })
        } else {
            None
        };

        let mut output = FloatImage::new(bg_w, bg_h, 4);
        let out_row_len = bg_w as usize * 4;
        let bg_raw = background.as_raw();
        let fg_raw = foreground.as_raw();
        let al_raw = alpha.as_raw();

        if out_row_len > 0 {
            output
                .as_raw_mut()
                .par_chunks_exact_mut(out_row_len)
                .enumerate()
                .for_each(|(y, out_row)| {
                    let bg_row = &bg_raw[y * bg_w as usize * bg_ch..][..bg_w as usize * bg_ch];

                    let foreground_y = y as i32 - position_y;
                    let fg_row = (foreground_y >= 0 && (foreground_y as u32) < fg_h)
                        .then(|| &fg_raw[foreground_y as usize * fg_w as usize * fg_ch..][..fg_w as usize * fg_ch]);
                    let al_row = ((y as u32) < al_h)
                        .then(|| &al_raw[y * al_w as usize * al_ch..][..al_w as usize * al_ch]);

                    for ((x, bg_px), out_px) in bg_row.chunks_exact(bg_ch).enumerate().zip(out_row.chunks_exact_mut(4)) {
                        let (br, bg_val, bb, ba) = expand_rgba(bg_px);

                        let foreground_x = x as i32 - position_x;
                        let fg_px = fg_row
                            .filter(|_| foreground_x >= 0 && (foreground_x as u32) < fg_w)
                            .map(|row| &row[foreground_x as usize * fg_ch..foreground_x as usize * fg_ch + fg_ch]);

                        if let Some(fg_px) = fg_px {
                            let (fr, fg, fb, fa) = expand_rgba(fg_px);
                            let mut blend_amount = amount;

                            // Modulate by alpha mask luminance
                            if let Some(al_row) = al_row {
                                if (x as u32) < al_w {
                                    let apx = &al_row[x * al_ch..x * al_ch + al_ch];
                                    let alpha_lum = if al_ch >= 3 { (apx[0] + apx[1] + apx[2]) / 3.0 } else { apx[0] };
                                    blend_amount = amount * alpha_lum;
                                }
                            }

                            let new_color = match &srgb_fast {
                                Some(fast) => blend_srgb_raw((br, bg_val, bb, ba), (fr, fg, fb, fa), fast, blend_amount),
                                None => {
                                    let background_color = Color::from_srgb_float(br, bg_val, bb, ba);
                                    let foreground_color = Color::from_srgb_float(fr, fg, fb, fa);
                                    blend_fn(background_color, foreground_color, &blend_mode, blend_amount).to_srgb_float()
                                }
                            };

                            out_px.copy_from_slice(&[new_color.0, new_color.1, new_color.2, new_color.3]);
                        } else {
                            out_px.copy_from_slice(&[br, bg_val, bb, ba]);
                        }
                    }
                });
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {value: Value::Image { data: Arc::new(output), change_id:get_id() }}],
        })
    }
}

/// Expands a pixel slice of 1–4 channels to an RGBA tuple.
///
/// 1ch: gray, opaque. 2ch: gray + alpha. 3ch: RGB, opaque. 4ch: RGBA.
#[inline]
fn expand_rgba(px: &[f32]) -> (f32, f32, f32, f32) {
    match px.len() {
        1 => (px[0], px[0], px[0], 1.0),
        2 => (px[0], px[0], px[0], px[1]),
        3 => (px[0], px[1], px[2], 1.0),
        _ => (px[0], px[1], px[2], px[3]),
    }
}

/// Per-pixel blend strategy for the sRGB fast path, selected once before the loop.
enum SrgbFastBlend {
    /// Over: lerp RGB by `amount * fg_alpha`, keep background alpha.
    Over,
    /// Linear interpolation of all channels (including alpha) by `amount`.
    Lerp,
    /// Photoshop-style per-channel formula from
    /// [`per_channel_fn`](crate::color::blend::per_channel_fn) (unit scale, zero offset).
    Ch(fn(f32, f32) -> f32),
}

/// Blends two raw sRGB pixels, producing the same result as
/// `Color::blend_srgb(bg, fg, mode, amount).to_srgb_float()` without the
/// intermediate `Color` values. The trailing clamps replicate `to_srgb_float`.
#[inline]
fn blend_srgb_raw(
    bg: (f32, f32, f32, f32),
    fg: (f32, f32, f32, f32),
    fast: &SrgbFastBlend,
    amount: f32,
) -> (f32, f32, f32, f32) {
    let (br, bg_val, bb, ba) = bg;
    let (fr, fg_val, fb, fa) = fg;
    match fast {
        SrgbFastBlend::Over => {
            let t = amount * fa;
            (
                lerp(br, fr, t).clamp(0.0, 1.0),
                lerp(bg_val, fg_val, t).clamp(0.0, 1.0),
                lerp(bb, fb, t).clamp(0.0, 1.0),
                ba.clamp(0.0, 1.0),
            )
        }
        SrgbFastBlend::Lerp => (
            lerp(br, fr, amount).clamp(0.0, 1.0),
            lerp(bg_val, fg_val, amount).clamp(0.0, 1.0),
            lerp(bb, fb, amount).clamp(0.0, 1.0),
            lerp(ba, fa, amount).clamp(0.0, 1.0),
        ),
        SrgbFastBlend::Ch(f) => {
            // Mirrors blend_ch with scale = 1.0, offset = 0.0.
            let t = amount * fa;
            let ch = |a: f32, b: f32| {
                lerp(a, f(a.clamp(0.0, 1.0), b.clamp(0.0, 1.0)), t).clamp(0.0, 1.0)
            };
            (ch(br, fr), ch(bg_val, fg_val), ch(bb, fb), ba.clamp(0.0, 1.0))
        }
    }
}

#[cfg(test)]
#[path = "blend_tests.rs"]
mod tests;
