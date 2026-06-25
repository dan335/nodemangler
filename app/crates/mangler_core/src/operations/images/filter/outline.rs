//! Mask outline / stroke.
//!
//! Given an input image treated as a mask (luminance is the mask if the
//! image has multiple channels), produces a stroked outline ring of
//! configurable thickness, position (outside, inside, centred), and color.
//! The output is a 4-channel RGBA image suitable for blending over the
//! original via the existing `blend` / `blit` nodes.
//!
//! Internally uses `separable_morphology` from `erode.rs` — dilate grows
//! the mask outward, erode shrinks it inward, and the difference between
//! the two defines where the ring lives.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::filter::erode::separable_morphology;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Produces a colored outline ring from a mask image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentOutline {}

impl OpImageAdjustmentOutline {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "outline".to_string(),
            description: "Stroke a mask edge with a configurable colour and thickness.".to_string(),
            help: "Threshold the input to a binary mask (>= 0.5 on luminance), then run morphological dilation and erosion to grow a thickened and shrunken copy; the difference between them defines the ring. The position input chooses which difference: `0` = outer stroke (dilate − mask), `1` = inner stroke (mask − erode), `2` = centred stroke (dilate − erode) straddling the original edge.\n\nOutput is always 4-channel RGBA with alpha matching the ring strength. The ring is fully opaque where the dilate/erode difference is saturated; antialiased samples that sit between 0 and 1 pass through to the alpha channel so the stroke has soft edges by default. Use `blit` or `blend` to composite the outline on top of the source image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask whose edges to stroke."),
            Input::new("thickness".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Stroke width in pixels (radius of dilate / erode)."),
            Input::new("position".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("0 = outer, 1 = inner, 2 = centred around the edge."),
            Input::new("color".to_string(), Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), None, None)
                .with_description("Color used for the outline strokes."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA image containing only the outline; alpha is zero where the source mask is unchanged."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let thickness_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let position_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let color_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Integer(position) = position_converted.unwrap() else { unreachable!() };
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let thickness = thickness.max(1) as i32;
        let position = position.clamp(0, 2);

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;

        // Project the input to a 1-channel mask image using luminance (Rec.709)
        // so morphology can run on a single scalar field instead of per-channel.
        let mut mask = FloatImage::new(w, h, 1);
        for y in 0..h {
            for x in 0..w {
                let px = data.get_pixel(x, y);
                let v = if ch >= 3 {
                    0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
                } else {
                    px[0]
                };
                mask.put_pixel(x, y, &[v]);
            }
        }

        // Compute dilated / eroded versions as needed by the chosen position.
        // Each call is O(r) per pixel thanks to the separable min/max passes.
        let dilated;
        let eroded;
        match position {
            0 => {
                // Outer only — need the dilate but not the erode.
                dilated = Some(separable_morphology(&mask, thickness, |a, b| a.max(b)));
                eroded = None;
            }
            1 => {
                // Inner only — need the erode but not the dilate.
                dilated = None;
                eroded = Some(separable_morphology(&mask, thickness, |a, b| a.min(b)));
            }
            _ => {
                // Centred — split the thickness roughly across both sides.
                let half = thickness.max(1);
                dilated = Some(separable_morphology(&mask, half, |a, b| a.max(b)));
                eroded = Some(separable_morphology(&mask, half, |a, b| a.min(b)));
            }
        }

        // Build the final RGBA stroke image. Alpha encodes stroke strength;
        // RGB is the chosen color multiplied by the user's color alpha so the
        // caller can fade the whole outline via the input color's alpha.
        let mut output = FloatImage::new(w, h, 4);
        for y in 0..h {
            for x in 0..w {
                let d = dilated.as_ref().map(|im| im.get_pixel(x, y)[0]);
                let e = eroded.as_ref().map(|im| im.get_pixel(x, y)[0]);
                let m = mask.get_pixel(x, y)[0];

                let alpha = match position {
                    0 => (d.unwrap() - m).clamp(0.0, 1.0),
                    1 => (m - e.unwrap()).clamp(0.0, 1.0),
                    _ => (d.unwrap() - e.unwrap()).clamp(0.0, 1.0),
                };

                let a = alpha * color.a;
                output.put_pixel(x, y, &[color.r, color.g, color.b, a]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "outline_tests.rs"]
mod tests;
