//! Border: pads an image with a solid mat, with an optional inner keyline.
//!
//! Unlike the resample-based nodes in this module, `border` grows the canvas
//! (like `crop` shrinks it) rather than resampling within fixed dimensions —
//! no bilinear sampling involved, just a solid fill plus a straight or
//! alpha-composited copy of the source into the middle.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Pads an image with a solid-colour border (mat), with an optional inner
/// keyline ring hugging the source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformBorder {}

impl OpImageTransformBorder {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "border".to_string(),
            description: "Pads the canvas with a solid-colour mat, with an optional thin keyline ring hugging the image.".to_string(),
            help: "Grows the canvas by `thickness` pixels on every side, filling the new border with `color` (a photo-mat effect; default white). `thickness` is authored in pixels at a 1024px reference and scaled to the source image, so the same value gives the same relative border at any resolution; the output width/height (also emitted as outputs) grow by twice the scaled thickness.\n\nFor images with an alpha channel, the source is alpha-composited over the border colour rather than pasted with its own alpha. With the default opaque mat that is `out_rgb = src_rgb·src_a + border_rgb·(1−src_a)` at `out_a = 1`, so the original image area reads as a solid mat, never a see-through hole. A translucent `color` composites accordingly instead of being forced opaque — set the border colour fully transparent and the node becomes plain transparent padding, leaving the source pixels exactly as they were. Images without alpha are copied straight through.\n\n`keyline` adds a thin ring of `keyline color` inside the border, immediately hugging the image edge — a common print-mat touch (e.g. a hairline rule between a photo and its mat). It only has an effect when `thickness` is greater than 0 (there's no border to draw it in otherwise) and is clamped to the border's own thickness so it can never spill past the mat's outer edge.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to add a border around."),
            Input::new("thickness".to_string(), Value::Integer(32), Some(InputSettings::Slider { range: (0.0, 256.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Border thickness in pixels at a 1024px reference (scales with image size); 0 = no border."),
            Input::new("color".to_string(), Value::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), None, None)
                .with_description("Colour of the border mat; a translucent colour composites through (fully transparent = plain padding)."),
            Input::new("keyline".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Thickness in pixels at a 1024px reference of a keyline ring drawn inside the border, hugging the image; 0 = off. Has no effect when thickness is 0."),
            Input::new("keyline color".to_string(), Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), None, None)
                .with_description("Colour of the keyline ring."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The bordered image."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Output image width in pixels (source width + 2 × thickness)."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Output image height in pixels (source height + 2 × thickness)."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let thickness_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let color_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);
        let keyline_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let keyline_color_converted = convert_input(inputs, 4, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Integer(keyline) = keyline_converted.unwrap() else { unreachable!() };
        let Value::Color(keyline_color) = keyline_color_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let nch = data.channels() as usize;

        let t = scale_to_resolution(thickness.max(0) as f32, width, height).round().max(0.0) as u32;

        if t == 0 {
            // No border to draw; a keyline alone has nothing to hug, so
            // passthrough entirely (see help).
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::clone(&data), change_id: get_id() } },
                    OutputResponse { value: Value::Integer(width as i32) },
                    OutputResponse { value: Value::Integer(height as i32) },
                ],
            });
        }

        // Keyline can never exceed the border it's drawn inside.
        let k = scale_to_resolution(keyline.max(0) as f32, width, height).round().max(0.0) as u32;
        let k = k.min(t);

        let border_px = color_to_pixel(color, nch);
        let keyline_px = color_to_pixel(keyline_color, nch);

        let out_w = width + 2 * t;
        let out_h = height + 2 * t;

        let mut output = FloatImage::from_pixel(out_w, out_h, nch as u32, &border_px);

        // Keyline ring: the k-pixel-thick band of border immediately
        // surrounding the source rectangle, inside the mat.
        if k > 0 {
            let rx0 = t - k;
            let ry0 = t - k;
            let rx1 = t + width + k; // exclusive
            let ry1 = t + height + k; // exclusive
            for y in ry0..ry1 {
                for x in rx0..rx1 {
                    let in_source = x >= t && x < t + width && y >= t && y < t + height;
                    if !in_source {
                        output.put_pixel(x, y, &keyline_px);
                    }
                }
            }
        }

        // Composite the source over the mat colour (straight alpha in, straight
        // alpha out). With the usual opaque mat `back` is `1 − src_a`, so this
        // is exactly `src_rgb·src_a + border_rgb·(1−src_a)` at `out_a = 1`; a
        // translucent mat composites properly instead of being forced opaque,
        // so a fully transparent `color` leaves the source untouched and the
        // node becomes plain transparent padding.
        let has_alpha = nch == 2 || nch == 4;
        let border_a = if has_alpha { border_px[nch - 1] } else { 1.0 };
        let mut px = vec![0.0f32; nch];
        for oy in 0..height {
            for ox in 0..width {
                let src = data.get_pixel(ox, oy);
                if has_alpha {
                    let a = src[nch - 1];
                    let back = border_a * (1.0 - a);
                    let out_a = a + back;
                    for c in 0..nch - 1 {
                        // Guarded like FloatImage::unpremultiply_alpha: with
                        // nothing visible there's no colour to recover.
                        px[c] = if out_a > 1e-6 { (src[c] * a + border_px[c] * back) / out_a } else { 0.0 };
                    }
                    px[nch - 1] = out_a;
                } else {
                    px.copy_from_slice(src);
                }
                output.put_pixel(t + ox, t + oy, &px);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
                OutputResponse { value: Value::Integer(out_w as i32) },
                OutputResponse { value: Value::Integer(out_h as i32) },
            ],
        })
    }
}

/// Reduces a `Color` to the image's channel layout (see `transform.rs`'s fill
/// colour handling for the same pattern).
fn color_to_pixel(color: Color, nch: usize) -> Vec<f32> {
    let luma = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
    match nch {
        1 => vec![luma],
        2 => vec![luma, color.a],
        3 => vec![color.r, color.g, color.b],
        _ => vec![color.r, color.g, color.b, color.a],
    }
}

#[cfg(test)]
#[path = "border_tests.rs"]
mod tests;
