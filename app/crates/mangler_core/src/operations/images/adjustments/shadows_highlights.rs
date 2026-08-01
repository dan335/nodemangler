//! Shadows/Highlights: Lightroom-style local tone recovery.
//!
//! Builds a blurred luminance mask (a rough "how bright is this region"
//! estimate) and uses it to gate four independent tone moves: lifting
//! shadows, recovering highlights, and two global endpoint trims (whites,
//! blacks). Colour is preserved by scaling all channels by the ratio between
//! the new and old luminance, the same trick used by `clarity`.
//!
//! This is a heuristic approximation of Adobe's Shadows/Highlights and
//! Lightroom's Basic panel sliders, not a calibrated tone-mapping model.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::operations::images::blur::blur::gaussian_blur_planar;
use crate::operations::numbers::image::luma_values;
use super::common::smoothstep;
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Local shadow/highlight recovery via a blurred-luminance mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentShadowsHighlights {}

fn slider(name: &str, desc: &str) -> Input {
    Input::new(
        name.to_string(),
        Value::Decimal(0.0),
        Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
        None,
    )
    .with_description(desc.to_string())
}

impl OpImageAdjustmentShadowsHighlights {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "shadows highlights".to_string(),
            description: "Lightroom-style shadow/highlight recovery using a blurred luminance mask.".to_string(),
            help: "Blurs the image's luminance at `radius` to build a local-brightness mask `M`, then applies four tone moves gated by that mask and by the (running) luminance itself:\n\n• shadows lifts dark regions (`M` near 0) toward mid-grey.\n• highlights pulls bright regions (`M` near 1) down (or up, for a negative slider) — positive brightens, negative recovers/compresses blown highlights.\n• whites is a global endpoint trim near pure white (no mask, just luminance).\n• blacks is a global endpoint trim near pure black.\n\nColour is preserved by scaling every colour channel by the ratio between the new and old luminance (like `clarity`); alpha is untouched. Radius is authored in pixels at a 1024px reference and scaled to the actual image. All four sliders at 0 leaves the image byte-identical. This is a heuristic local-tone tool, not a calibrated HDR/tone-mapping model.".to_string(),
        }
    }

    /// Creates the input ports: image, four tone sliders, and mask radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to recover shadow/highlight detail in."),
            slider("shadows", "Lifts dark regions (masked by local brightness); negative crushes them further."),
            slider("highlights", "Recovers/compresses bright regions (masked by local brightness); negative darkens, positive brightens."),
            slider("whites", "Global endpoint trim near pure white, not masked by locality."),
            slider("blacks", "Global endpoint trim near pure black, not masked by locality."),
            Input::new("radius".to_string(), Value::Decimal(32.0), Some(InputSettings::Slider { range: (1.0, 256.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Mask blur radius in pixels at a 1024px reference (scales with image size)."),
        ]
    }

    /// Creates the output port: the recovered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with shadow/highlight recovery applied, alpha preserved."),
        ]
    }

    /// Executes the shadows/highlights recovery.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let shadows_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let highlights_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let whites_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let blacks_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(shadows) = shadows_converted.unwrap() else { unreachable!() };
        let Value::Decimal(highlights) = highlights_converted.unwrap() else { unreachable!() };
        let Value::Decimal(whites) = whites_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blacks) = blacks_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let shadows = shadows as f32;
        let highlights = highlights as f32;
        let whites = whites as f32;
        let blacks = blacks as f32;

        // Nothing to do: every tone move is a no-op regardless of radius.
        if shadows == 0.0 && highlights == 0.0 && whites == 0.0 && blacks == 0.0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let mut result = (*data).clone();
        let (w, h) = result.dimensions();
        let ch = result.channels() as usize;
        let wu = w as usize;

        let sigma = scale_to_resolution(radius as f32, w, h).max(0.0);

        let luma = luma_values(&result);
        let mask = gaussian_blur_planar(&luma, w, h, sigma);

        for y in 0..h {
            for x in 0..w {
                let i = y as usize * wu + x as usize;
                let l0 = luma[i];
                let m = mask[i];

                let mut l = l0;
                // Shadow lift, gated by the "this region is dark" mask weight.
                let w_sh = 1.0 - smoothstep(0.0, 0.5, m);
                l += shadows * w_sh * (1.0 - l) * 0.6;
                // Highlight recovery, gated by the "this region is bright" mask weight.
                let w_hi = smoothstep(0.5, 1.0, m);
                l += highlights * w_hi * l * 0.6;
                // Global endpoint trims, driven by luminance directly (no mask).
                l += whites * smoothstep(0.7, 1.0, l) * 0.3;
                l += blacks * (1.0 - smoothstep(0.0, 0.3, l)) * 0.3;
                let new_luma = l;

                let px = result.get_pixel_mut(x, y);
                if ch >= 3 {
                    // Preserve hue/chroma: scale colour channels by the luma ratio.
                    let scale = if l0.abs() > 1e-5 { new_luma / l0 } else { 1.0 };
                    let color_ch = if ch == 4 { 3 } else { ch };
                    for c in 0..color_ch {
                        px[c] = (px[c] * scale).clamp(0.0, 1.0);
                    }
                    // Alpha (channel 3 for RGBA) is left untouched.
                } else {
                    // Single/luma+alpha image: channel 0 IS the luma.
                    px[0] = new_luma.clamp(0.0, 1.0);
                }
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "shadows_highlights_tests.rs"]
mod tests;
