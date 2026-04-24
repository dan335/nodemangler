//! Toon / cel-shade filter operation for images.
//!
//! Three-stage stylization pipeline tuned for an actual cel-shaded look:
//!
//! 1. **Pre-smooth** — small box blur to flatten texture and noise so the
//!    quantization step produces clean bands instead of speckle.
//! 2. **Quantize value, preserve hue** — round-trip each pixel through HSV,
//!    round only the value (brightness) to one of N bins, convert back.
//!    Smooth color gradients become clean tonal bands of the original hue
//!    instead of the disjoint hue shifts a per-channel posterize would give.
//!    HSV is preferred over HSL here because pure saturated colors live at
//!    `V=1, S=1` so quantizing V down keeps the colour pure (darker red),
//!    while HSL would force them off the saturation cap and desaturate.
//! 3. **Cel-boundary edge overlay** — after quantization, the only gradients
//!    that exist in the image are the cel-band boundaries. Detecting edges
//!    directly on the quantized output gives clean, threshold-free outlines
//!    that follow shading bands exactly. A small box blur on the binary edge
//!    mask provides controllable thickness with built-in anti-aliasing.
//!    DoG was tried first but on natural images its response was either too
//!    subtle to threshold reliably or fired on irrelevant texture detail.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Toon / cel-shade filter: pre-smooth, quantize lightness, overlay DoG edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentToon {}

impl OpImageAdjustmentToon {
    /// Returns the node metadata (name and description) for the toon operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "toon".to_string(),
            description: "Cel-shade effect: pre-smooth, quantize lightness (preserving hue), and overlay DoG edges.".to_string(),
            help: "Three-stage cel-shade pipeline. First a small box blur flattens texture so quantization produces clean bands. Then each pixel round-trips through HSV and only the V channel is snapped to `levels` bins — saturated colors stay saturated (darker red rather than desaturated) where a per-channel posterize would shift hue.\n\nFinally cel-boundary outlines are detected on the quantized V buffer (no threshold needed since V is the only piecewise-constant dimension), thickened with a box-blur plus smoothstep, and composited using `edge color` at `edge strength`.".to_string(),
        }
    }

    /// Creates the input ports for the toon operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to cel-shade with quantized lightness bands."),
            // number of lightness bands. 4 = classic shadow / midtone / highlight / specular
            Input::new("levels".to_string(), Value::Integer(4), Some(InputSettings::DragValue { speed: None, clamp: Some((2.0, 8.0)) }), None)
                .with_description("Number of lightness bands; 4 gives classic shadow/midtone/highlight/specular."),
            // pre-blur radius (in pixels) applied to the image before quantization;
            // 0 disables smoothing, default 2 is a good starting point for most photos
            Input::new("smoothing".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 5.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Pre-quantization blur radius in pixels; larger values flatten texture before banding."),
            // blur radius (in pixels) applied to the binary cel-boundary mask;
            // 0 = sharp 1-pixel lines, larger = thicker / softer outlines.
            // Integer because the underlying box-blur radius is an integer pixel count.
            Input::new("edge thickness".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 5.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Blur radius applied to the cel-boundary mask; 0 gives sharp 1-pixel outlines."),
            // color drawn on detected edges
            Input::new("edge color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color drawn along the detected cel-band outlines."),
            // global multiplier on how strongly edges replace the underlying color
            Input::new("edge strength".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the outline overlay; 0 disables edges, 1 fully replaces pixels with edge color."),
        ]
    }

    /// Creates the output port: the toon-shaded image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Cel-shaded image with banded lightness and overlaid cel-boundary outlines."),
        ]
    }

    /// Executes the toon filter pipeline.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let smoothing_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let thickness_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let edge_color_converted = convert_input(inputs, 4, ValueType::Color, &mut input_errors);
        let edge_strength_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };
        let Value::Integer(smoothing) = smoothing_converted.unwrap() else { unreachable!() };
        let Value::Integer(edge_thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Color(edge_color) = edge_color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(edge_strength) = edge_strength_converted.unwrap() else { unreachable!() };

        let levels = (levels.max(2)) as f32;
        let steps = levels - 1.0;
        let smoothing = smoothing.max(0) as usize;
        let edge_thickness = edge_thickness.max(0);
        let edge_strength = edge_strength.clamp(0.0, 1.0);
        let (edge_r, edge_g, edge_b, _edge_a) = edge_color.to_srgb_float();

        let (width, height) = data.dimensions();
        let w = width as usize;
        let h = height as usize;
        let n = w * h;
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };

        // Extract per-channel planes for easy box-blur math.
        let mut planes: Vec<Vec<f32>> = vec![vec![0.0; n]; ch];
        for y in 0..h {
            for x in 0..w {
                let pixel = data.get_pixel(x as u32, y as u32);
                let idx = y * w + x;
                for c in 0..ch {
                    planes[c][idx] = pixel[c];
                }
            }
        }

        // ---- Step 1: pre-smooth the color planes (alpha left untouched) ----
        let smoothed: Vec<Vec<f32>> = if smoothing == 0 {
            planes.clone()
        } else {
            let mut out = planes.clone();
            for c in 0..color_ch {
                out[c] = box_blur_2d(&planes[c], w, h, smoothing);
            }
            out
        };

        // ---- Step 2: quantize value (HSV) with hue preserved ----
        // For RGB images we round-trip each smoothed pixel through HSV, snap V
        // (brightness) to a band, and convert back. For grayscale we just
        // quantize the single channel directly.
        //
        // We also keep the quantized V values in a separate buffer for the edge
        // pass below — H and S vary continuously per pixel so the final RGB is
        // not piecewise-constant within a band, and detecting edges directly on
        // RGB would fire on small in-band hue/saturation jitter.
        let mut quantized: Vec<Vec<f32>> = vec![vec![0.0; n]; ch];
        let mut v_quantized = vec![0.0f32; n];
        if color_ch >= 3 {
            for i in 0..n {
                // build a Color from the smoothed sRGB pixel; alpha is irrelevant here
                let c = Color::from_srgb_float(smoothed[0][i], smoothed[1][i], smoothed[2][i], 1.0);
                let (hue, sat, val, _) = c.to_hsv();
                let v_q = ((val * steps + 0.5).floor() / steps).clamp(0.0, 1.0);
                v_quantized[i] = v_q;
                let back = Color::from_hsv(hue, sat, v_q, 1.0);
                let (r, g, b, _) = back.to_srgb_float();
                quantized[0][i] = r.clamp(0.0, 1.0);
                quantized[1][i] = g.clamp(0.0, 1.0);
                quantized[2][i] = b.clamp(0.0, 1.0);
            }
        } else {
            for i in 0..n {
                let v = smoothed[0][i];
                let v_q = ((v * steps + 0.5).floor() / steps).clamp(0.0, 1.0);
                v_quantized[i] = v_q;
                quantized[0][i] = v_q;
            }
        }
        // alpha passes through unchanged from the source
        if has_alpha {
            quantized[ch - 1] = planes[ch - 1].clone();
        }

        // ---- Step 3: detect cel-band edges on the quantized V buffer ----
        // V is the only dimension that's actually snapped to discrete bands;
        // H and S vary continuously. So the V_q buffer is the one piece of the
        // pipeline that's genuinely piecewise-constant, and any change in it is
        // a real cel-band boundary. No threshold needed.
        let mut binary_edges = vec![0.0f32; n];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let center = v_quantized[i];
                let left   = if x > 0     { v_quantized[y * w + x - 1] } else { center };
                let right  = if x + 1 < w { v_quantized[y * w + x + 1] } else { center };
                let up     = if y > 0     { v_quantized[(y - 1) * w + x] } else { center };
                let down   = if y + 1 < h { v_quantized[(y + 1) * w + x] } else { center };
                if left != center || right != center || up != center || down != center {
                    binary_edges[i] = 1.0;
                }
            }
        }

        // Thicken / soften the edge mask. A box blur of a binary mask gives a
        // linear falloff over the kernel; combined with a smoothstep this turns
        // 1-pixel binary edges into smooth, anti-aliased outlines whose width
        // tracks the radius. radius==0 keeps the original sharp 1-pixel mask.
        let blur_radius = edge_thickness as usize;
        let edge_mask = if blur_radius == 0 {
            binary_edges
        } else {
            let blurred = box_blur_2d(&binary_edges, w, h, blur_radius);
            // remap so the inner core of the edge stays at 1 but the soft tail
            // falls off via smoothstep — gives a clean visible outline rather
            // than a uniformly faint band the width of the blur kernel
            blurred.into_iter().map(|v| {
                let t = (v * 4.0).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }).collect::<Vec<f32>>()
        };

        // ---- Composite: quantized color, then edge color blended on top ----
        let mut pixels = vec![0.0f32; n * ch];
        for i in 0..n {
            let blend = (edge_mask[i] * edge_strength).clamp(0.0, 1.0);
            if color_ch >= 3 {
                pixels[i * ch    ] = quantized[0][i] * (1.0 - blend) + edge_r * blend;
                pixels[i * ch + 1] = quantized[1][i] * (1.0 - blend) + edge_g * blend;
                pixels[i * ch + 2] = quantized[2][i] * (1.0 - blend) + edge_b * blend;
            } else {
                let edge_lum = 0.2126 * edge_r + 0.7152 * edge_g + 0.0722 * edge_b;
                pixels[i * ch] = quantized[0][i] * (1.0 - blend) + edge_lum * blend;
            }
            if has_alpha {
                pixels[i * ch + ch - 1] = quantized[ch - 1][i];
            }
        }

        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Separable 2D box blur with edge clamping. O(1) per pixel via 1D prefix sums.
///
/// Same algorithm as the helper in `guided.rs` — copied here rather than shared
/// because it's a small primitive and an early dependency split would add
/// surface area. If a third filter wants it, extract to `filter/box_blur.rs`.
fn box_blur_2d(input: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    // horizontal pass: prefix-sum each row, then read out box mean at every x
    let mut h_pass = vec![0.0f32; input.len()];
    let mut prefix = vec![0.0f64; width + 1];
    for y in 0..height {
        let row_start = y * width;
        prefix[0] = 0.0;
        for x in 0..width {
            prefix[x + 1] = prefix[x] + input[row_start + x] as f64;
        }
        for x in 0..width {
            let lo = x.saturating_sub(radius);
            let hi = (x + radius + 1).min(width);
            let cnt = (hi - lo) as f64;
            h_pass[row_start + x] = ((prefix[hi] - prefix[lo]) / cnt) as f32;
        }
    }

    // vertical pass: same idea over columns
    let mut out = vec![0.0f32; input.len()];
    let mut col_prefix = vec![0.0f64; height + 1];
    for x in 0..width {
        col_prefix[0] = 0.0;
        for y in 0..height {
            col_prefix[y + 1] = col_prefix[y] + h_pass[y * width + x] as f64;
        }
        for y in 0..height {
            let lo = y.saturating_sub(radius);
            let hi = (y + radius + 1).min(height);
            let cnt = (hi - lo) as f64;
            out[y * width + x] = ((col_prefix[hi] - col_prefix[lo]) / cnt) as f32;
        }
    }

    out
}

#[cfg(test)]
#[path = "toon_tests.rs"]
mod tests;
