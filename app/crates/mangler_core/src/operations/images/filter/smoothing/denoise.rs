//! Denoise: separate luminance and chrominance smoothing in YCbCr.
//!
//! Photographic noise is not one signal: fine luminance grain wants a small
//! radius and a light touch (over-smoothing it plasticizes detail), while
//! chroma blotches are low-frequency and can take a much larger radius without
//! visibly softening the picture. Raw converters therefore split the image
//! into luma and chroma and denoise each with its own strength and radius.
//!
//! Both passes use the guided filter (He et al. 2010), which is edge-preserving
//! and O(1) per pixel regardless of radius. The chroma planes are filtered with
//! **luma as the guide** — the classic joint-filtering trick: chroma has little
//! usable structure of its own, so borrowing luma's edges keeps colour from
//! bleeding across boundaries even at large radii.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::operations::images::adjustments::common::{rgb_to_ycbcr, ycbcr_to_rgb};
use crate::operations::images::filter::smoothing::guided::{guide_stats, guided_filter_plane, guided_filter_plane_with_stats};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Scale from the luminance strength slider to the guided filter's epsilon.
/// Squared because epsilon is compared against a *variance*, so this reads as
/// "smooth detail whose contrast is below `luminance * 0.08`".
const LUMA_EPS_SCALE: f32 = 0.08;

/// Same idea for chroma, but chroma noise is coarser and safer to crush, so
/// the same slider position buys a much larger epsilon.
const CHROMA_EPS_SCALE: f32 = 0.25;

/// Floor on epsilon so the local linear fit never divides by zero.
const EPS_FLOOR: f32 = 1e-6;

/// Blends a filtered plane back over the original by `strength`, giving a
/// smooth ramp from "untouched" to "fully filtered".
#[inline]
fn blend(orig: &[f32], filtered: &[f32], strength: f32) -> Vec<f32> {
    (0..orig.len()).map(|i| orig[i] + strength * (filtered[i] - orig[i])).collect()
}

/// Luminance/chrominance denoising operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDenoise {}

impl OpImageAdjustmentDenoise {
    /// Returns the node metadata (name, description, help) for the denoise operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "denoise".to_string(),
            description: "Separate luminance and chroma denoising with edge-preserving guided filters.".to_string(),
            help: "Separate luminance/chrominance denoising in YCbCr using edge-preserving guided filters (He et al. 2010) with luma-guided chroma smoothing.\n\nThe image is converted to BT.709 YCbCr. The luma plane is filtered self-guided at `luminance radius`, with the filter's epsilon driven by `luminance` — higher strength smooths detail of greater contrast. The Cb/Cr planes are filtered at `chroma radius` using the *original luma* as the guide, so colour edges stay locked to luminance edges even at radii large enough to erase chroma blotches. Each plane is then blended back over its original by its strength slider, so the sliders ramp smoothly from no effect to fully filtered.\n\nUse a small luminance radius and a light strength (over-smoothing luma looks plastic) with a large chroma radius and a heavier strength (chroma noise is coarse and rarely carries detail). Radii are authored in pixels at a 1024px reference and scale with the image. Grayscale inputs run the luminance path only; alpha is never denoised and passes through unchanged. Both strengths at 0 passes the image through untouched.".to_string(),
        }
    }

    /// Creates the input ports: image, then the luma and chroma strength/radius pairs.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to denoise."),
            Input::new("luminance".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance denoise strength; higher smooths more detail (and risks a plastic look). 0 leaves luma untouched."),
            Input::new("luminance radius".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Luminance filter radius in pixels at a 1024px reference (scales with image size); keep small so fine detail survives."),
            Input::new("chroma".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Chroma denoise strength; removes coloured blotches. 0 leaves the colour planes untouched."),
            Input::new("chroma radius".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Chroma filter radius in pixels at a 1024px reference (scales with image size); can be large — luma guides the edges."),
        ]
    }

    /// Creates the output port: the denoised image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Denoised image, alpha preserved."),
        ]
    }

    /// Executes the denoise operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let luma_strength_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let luma_radius_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let chroma_strength_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let chroma_radius_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);

        // Return if any conversion failed.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Extract values.
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(luma_strength) = luma_strength_converted.unwrap() else { unreachable!() };
        let Value::Integer(luma_radius) = luma_radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(chroma_strength) = chroma_strength_converted.unwrap() else { unreachable!() };
        let Value::Integer(chroma_radius) = chroma_radius_converted.unwrap() else { unreachable!() };

        let luma_strength = (luma_strength as f32).clamp(0.0, 1.0);
        let chroma_strength = (chroma_strength as f32).clamp(0.0, 1.0);

        // Nothing asked for: hand the original Arc straight back.
        if luma_strength <= 0.0 && chroma_strength <= 0.0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let (w, h) = data.dimensions();
        let wu = w as usize;
        let hu = h as usize;
        let n = wu * hu;
        let ch = data.channels() as usize;
        let is_color = ch >= 3;

        // Radii are authored in reference pixels (at 1024px) and scaled to the
        // actual image so the same setting denoises the same relative scale.
        let r_luma = scale_to_resolution(luma_radius.max(1) as f32, w, h).round().max(1.0) as usize;
        let r_chroma = scale_to_resolution(chroma_radius.max(1) as f32, w, h).round().max(1.0) as usize;

        // Epsilon is a variance threshold: detail below it gets averaged away.
        let eps_luma = (luma_strength * LUMA_EPS_SCALE).powi(2) + EPS_FLOOR;
        let eps_chroma = (chroma_strength * CHROMA_EPS_SCALE).powi(2) + EPS_FLOOR;

        // Split into planes. Grayscale (1 or 2 channels) has no chroma, so
        // channel 0 doubles as the luma plane and the chroma path is skipped.
        let mut y_plane = vec![0.0f32; n];
        let mut cb_plane = vec![0.0f32; n];
        let mut cr_plane = vec![0.0f32; n];
        for (i, px) in data.pixels().enumerate() {
            if is_color {
                let (y, cb, cr) = rgb_to_ycbcr(px[0], px[1], px[2]);
                y_plane[i] = y;
                cb_plane[i] = cb;
                cr_plane[i] = cr;
            } else {
                y_plane[i] = px[0];
            }
        }

        // Luma: self-guided at its own (small) radius. Strength 0 short-circuits
        // the filter entirely rather than blending a result nobody asked for.
        let y_out = if luma_strength > 0.0 {
            let filtered = guided_filter_plane(&y_plane, &y_plane, wu, hu, r_luma, eps_luma);
            blend(&y_plane, &filtered, luma_strength)
        } else {
            y_plane.clone()
        };

        // Chroma: both planes share one guide (the *original* luma) at one
        // radius, so the guide statistics are computed once and reused.
        let (cb_out, cr_out) = if is_color && chroma_strength > 0.0 {
            let stats = guide_stats(&y_plane, wu, hu, r_chroma);
            let cb_f = guided_filter_plane_with_stats(&cb_plane, &y_plane, &stats, wu, hu, r_chroma, eps_chroma);
            let cr_f = guided_filter_plane_with_stats(&cr_plane, &y_plane, &stats, wu, hu, r_chroma, eps_chroma);
            (blend(&cb_plane, &cb_f, chroma_strength), blend(&cr_plane, &cr_f, chroma_strength))
        } else {
            (cb_plane, cr_plane)
        };

        // Recombine, leaving alpha exactly as it came in.
        let mut result = (*data).clone();
        for (i, px) in result.pixels_mut().enumerate() {
            if is_color {
                let (r, g, b) = ycbcr_to_rgb(y_out[i], cb_out[i], cr_out[i]);
                px[0] = r.clamp(0.0, 1.0);
                px[1] = g.clamp(0.0, 1.0);
                px[2] = b.clamp(0.0, 1.0);
                // Alpha (channel 3 on RGBA) is untouched — never denoise alpha.
            } else {
                px[0] = y_out[i].clamp(0.0, 1.0);
                // Channel 1 on a gray+alpha image is alpha; untouched.
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
#[path = "denoise_tests.rs"]
mod tests;
