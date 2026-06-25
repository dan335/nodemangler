//! Color replacement operation.
//!
//! Swaps pixels that match a source color (with tolerance + soft falloff)
//! for a target color. Internally reuses the same smoothstep-masked selection
//! as `color_to_mask` and then linearly interpolates between the original
//! pixel and the replacement according to the mask strength.

use crate::color::Color;
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

/// Replaces pixels that match a source color (with tolerance) with a new color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentReplaceColor {}

impl OpImageAdjustmentReplaceColor {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "replace color".to_string(),
            description: "Swaps pixels close to a source color for a target color, with soft falloff.".to_string(),
            help: "Per pixel, computes the normalised RGB Euclidean distance to the source color (same metric as color-to-mask) and derives a selection weight that is 1 below `tolerance`, 0 past `tolerance + softness`, and smoothstep-faded between. Output = lerp(source pixel, target color, weight).\n\nAlpha channel and channel count of the input are preserved; RGB values from the target color are used for the replacement. Grayscale (1 / 2 channel) inputs match on luminance only and blend to the target color's luminance.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose matching pixels will be recolored."),
            Input::new("from".to_string(), Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }), None, None)
                .with_description("Color to find; pixels near this are replaced."),
            Input::new("to".to_string(), Value::Color(Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }), None, None)
                .with_description("Replacement color used where the mask is 1."),
            Input::new("tolerance".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Maximum normalised color distance still counted as a full match."),
            Input::new("softness".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Width of the smoothstep falloff past tolerance; zero gives a hard edge."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with matched pixels lerped toward the target color."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let from_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let to_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);
        let tolerance_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let softness_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Color(from) = from_converted.unwrap() else { unreachable!() };
        let Value::Color(to) = to_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tolerance) = tolerance_converted.unwrap() else { unreachable!() };
        let Value::Decimal(softness) = softness_converted.unwrap() else { unreachable!() };

        let tolerance = tolerance.clamp(0.0, 1.0);
        let softness = softness.max(0.0);
        let outer = tolerance + softness;

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let mut output = FloatImage::new(w, h, ch as u32);

        let norm = 3.0_f32.sqrt();
        // Pre-compute grayscale equivalents for 1/2-channel paths.
        let from_luma = 0.2126 * from.r + 0.7152 * from.g + 0.0722 * from.b;
        let to_luma = 0.2126 * to.r + 0.7152 * to.g + 0.0722 * to.b;

        let mut buf = [0.0f32; 4];
        for y in 0..h {
            for x in 0..w {
                let src = data.get_pixel(x, y);
                let weight = if ch >= 3 {
                    let dr = src[0] - from.r;
                    let dg = src[1] - from.g;
                    let db = src[2] - from.b;
                    let dist = (dr * dr + dg * dg + db * db).sqrt() / norm;
                    smooth_weight(dist, tolerance, outer)
                } else {
                    let dist = (src[0] - from_luma).abs();
                    smooth_weight(dist, tolerance, outer)
                };

                if ch >= 3 {
                    // RGB replace; alpha (if any) is preserved from the source.
                    buf[0] = src[0] + (to.r - src[0]) * weight;
                    buf[1] = src[1] + (to.g - src[1]) * weight;
                    buf[2] = src[2] + (to.b - src[2]) * weight;
                    if ch == 4 {
                        buf[3] = src[3];
                    }
                } else {
                    // Grayscale; blend luminances.
                    buf[0] = src[0] + (to_luma - src[0]) * weight;
                    if ch == 2 {
                        buf[1] = src[1];
                    }
                }
                output.put_pixel(x, y, &buf[..ch]);
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

/// Soft selection weight matching `color_to_mask::smooth_select`.
#[inline]
fn smooth_weight(d: f32, tol: f32, outer: f32) -> f32 {
    if d <= tol { return 1.0; }
    if d >= outer || outer <= tol { return 0.0; }
    let t = (d - tol) / (outer - tol);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

#[cfg(test)]
#[path = "replace_color_tests.rs"]
mod tests;
