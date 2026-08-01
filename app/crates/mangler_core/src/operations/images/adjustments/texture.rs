//! Texture: fine-detail local contrast using an edge-preserving guided filter.
//!
//! Lightroom's "Texture" slider boosts (or smooths) the *fine* detail band of
//! an image without the halos a plain unsharp mask produces at strong edges.
//! The base layer here is a small-radius guided filter (He et al. 2010) of the
//! luminance, guided by the luminance itself: it follows edges instead of
//! averaging across them, so `detail = luma - base` contains texture but very
//! little edge overshoot. The detail is then added back scaled by `amount`.
//!
//! Unlike `clarity`, there is **no midtone weighting** — texture acts evenly
//! across the tonal range and at a much smaller radius. Colour is preserved by
//! scaling every colour channel by the same `new_luma / luma` ratio; alpha is
//! untouched.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::operations::images::filter::smoothing::guided::guided_filter_plane;
use crate::operations::numbers::image::luma_values;
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Edge-preservation regularizer for the base layer. Small enough that the
/// guided filter tracks real edges tightly (so they end up in the base, not
/// the detail), large enough to still smooth away fine texture.
const BASE_EPS: f32 = 1e-3;

/// Gain applied to `amount` so the slider's ±1 range spans a useful strength.
const DETAIL_GAIN: f32 = 2.0;

/// Fine-detail local contrast (texture) adjustment operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentTexture {}

impl OpImageAdjustmentTexture {
    /// Returns the node metadata (name, description, help) for the texture operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "texture".to_string(),
            description: "Fine-detail local contrast; positive brings out texture, negative smooths it.".to_string(),
            help: "Fine-detail local contrast using an edge-preserving guided filter (He et al. 2010) — Lightroom-Texture-style; edge-aware so it avoids halos.\n\nThe image's luminance is filtered with a small-radius self-guided filter to build a base layer, and the residual `detail = luma - base` is added back scaled by `amount`. Because the guided filter follows edges rather than averaging across them, strong edges land in the base layer and stay out of the detail — so boosting texture doesn't ring around them the way an unsharp mask does.\n\nUnlike `clarity` there is no midtone weighting: texture applies evenly across the tonal range, at a much smaller radius. Positive amounts bring out skin/fabric/bark texture, negative amounts smooth it while leaving edges crisp. `size` is authored in pixels at a 1024px reference and scaled to the actual image, so the same value gives the same relative effect at any resolution. Colour is preserved by scaling all colour channels by the new/old luminance ratio; alpha is untouched. `amount` 0 passes the image through unchanged.".to_string(),
        }
    }

    /// Creates the input ports: source image, signed amount, and detail size.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to adjust."),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Texture strength; positive brings out fine detail, negative smooths it, 0 leaves the image unchanged."),
            Input::new("size".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Detail scale in pixels at a 1024px reference (scales with image size); larger values act on coarser texture."),
        ]
    }

    /// Creates the output port: the texture-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with fine-detail local contrast adjusted, alpha preserved."),
        ]
    }

    /// Executes the texture operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let size_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        // Return if any conversion failed.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Extract values.
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::Integer(size) = size_converted.unwrap() else { unreachable!() };

        let amount = amount as f32;

        // Degenerate amount: nothing to do, hand the original Arc straight back.
        if amount == 0.0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let (w, h) = data.dimensions();
        let wu = w as usize;
        let hu = h as usize;
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // `size` is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the detail band stays the same relative scale.
        let radius = scale_to_resolution(size.max(1) as f32, w, h).round().max(1.0) as usize;

        let mut result = (*data).clone();

        // Luminance plane and its edge-preserving base layer. Self-guided, so
        // real edges are reproduced by the base and stay out of the detail.
        let luma = luma_values(&result);
        let base = guided_filter_plane(&luma, &luma, wu, hu, radius, BASE_EPS);

        for (i, px) in result.pixels_mut().enumerate() {
            let l = luma[i];
            // Fine-detail residual, free of the edge overshoot an unsharp mask
            // would carry. No midtone weighting — that's `clarity`'s job.
            let detail = l - base[i];
            let new_luma = (l + amount * detail * DETAIL_GAIN).clamp(0.0, 1.0);

            if color_ch >= 3 {
                // Preserve hue: scale all colour channels by the luma ratio.
                let scale = if l.abs() > 1e-5 { new_luma / l } else { 1.0 };
                for val in px.iter_mut().take(color_ch) {
                    *val = (*val * scale).clamp(0.0, 1.0);
                }
                // Alpha (channel 3 on RGBA) is left untouched.
            } else {
                // Grayscale (+ optional alpha): the luma *is* the channel.
                px[0] = new_luma;
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "texture_tests.rs"]
mod tests;
