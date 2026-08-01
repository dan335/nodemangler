//! Tone equalizer: zone-based exposure adjustment driven by a drawn curve.
//!
//! Ansel Adams' zone system, as a slider bank: split the image into
//! log2-luminance zones and push each zone's exposure up or down
//! independently. darktable's tone equalizer does this with a row of sliders
//! plus a guided-filter "detail preservation" control; here the slider bank is
//! a drawn curve — its x axis is the zone (−8 EV on the left, 0 EV on the
//! right) and its y axis is the exposure gain applied there (centre = no
//! change, ±4 EV at the extremes).
//!
//! The gain is looked up per pixel through a *smoothed* luminance mask rather
//! than the raw luminance, so the adjustment behaves like large-scale
//! dodging/burning instead of a per-pixel tone curve. The mask is smoothed
//! with an edge-preserving guided filter (He et al. 2010), which is what stops
//! halos appearing along high-contrast boundaries.
//!
//! Heuristic reimplementation — the zone mapping and mask are in the spirit of
//! darktable's module, not a port of it.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::operations::images::filter::smoothing::guided::guided_filter_plane;
use crate::operations::images::tone_curve::{flat_tone_curve, optional_lut_vs, sample_lut};
use crate::operations::numbers::image::luma_values;
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Darkest zone the curve addresses, in EV below white. The zone coordinate
/// maps `[-EV_RANGE .. 0] EV` onto `[0 .. 1]` along the curve's x axis.
const EV_RANGE: f32 = 8.0;

/// Full-scale gain, in EV, at the curve's extremes: a curve value of 1 gives
/// `+EV_RANGE/2`, a value of 0 gives `-EV_RANGE/2`.
const GAIN_EV_SPAN: f32 = 8.0;

/// Zone-based exposure adjustment operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentToneEqualizer {}

impl OpImageAdjustmentToneEqualizer {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tone equalizer".to_string(),
            description: "Per-zone exposure: a drawn curve maps luminance zones to exposure gain.".to_string(),
            help: "Zone-based exposure adjustment in the spirit of darktable's tone equalizer: a drawn curve maps log₂-luminance zones (−8..0 EV) to exposure gain (±4 EV), applied through an edge-preserving smoothed luminance mask. Heuristic reimplementation.\n\nThe curve's horizontal axis is the zone: the left edge is 8 stops below white (deep shadows), the right edge is white. Its vertical axis is the gain applied in that zone — the flat default line down the middle means 0 EV everywhere, so the image is passed through untouched. Raising the left half lifts shadows; lowering the right half pulls highlights back.\n\nEach pixel's zone is read from a *smoothed* luminance mask, not its own value, so the result reads as large-scale dodging and burning rather than a tone curve. `smoothing` sets the mask radius (in pixels at a 1024px reference, scaling with the image) and `detail preservation` sets how tightly the mask follows edges: high values keep the mask sharp at edges (less halo, more local contrast retained), low values let it blur across them. Alpha is untouched.".to_string(),
        }
    }

    /// Creates the input ports: image, the zone curve, and the two mask controls.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to adjust."),
            // Same shape as `tone_curve_input`, but the untouched default is
            // the flat mid line (0 EV everywhere) rather than the identity
            // diagonal — this curve is a signed gain, not a value remap.
            Input::new("zones".to_string(), Value::Curve(flat_tone_curve()), Some(InputSettings::ToneCurve), None)
                .with_description("Exposure gain per luminance zone: x is the zone (left = -8 EV, right = white), y is the gain (centre = 0 EV, top = +4 EV, bottom = -4 EV)."),
            Input::new("smoothing".to_string(), Value::Integer(64), Some(InputSettings::Slider { range: (4.0, 256.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Radius of the luminance mask in pixels at a 1024px reference (scales with image size); larger values give broader, more gradual dodging and burning."),
            Input::new("detail preservation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How tightly the mask follows edges; higher keeps edges sharp in the mask (fewer halos), lower lets the mask blur across them."),
        ]
    }

    /// Creates the output port: the tone-equalized image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with per-zone exposure applied, alpha preserved."),
        ]
    }

    /// Executes the tone equalizer.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let curve_converted = convert_input(inputs, 1, ValueType::Curve, &mut input_errors);
        let smoothing_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let detail_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // Return if any conversion failed.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Extract values.
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Integer(smoothing) = smoothing_converted.unwrap() else { unreachable!() };
        let Value::Decimal(detail) = detail_converted.unwrap() else { unreachable!() };

        // The untouched flat default means 0 EV in every zone — pass the
        // original Arc through rather than paying for the mask.
        let Some(lut) = optional_lut_vs(&curve, &flat_tone_curve()) else {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        };

        let detail = (detail as f32).clamp(0.0, 1.0);

        let (w, h) = data.dimensions();
        let wu = w as usize;
        let hu = h as usize;
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Mask radius is authored in reference pixels (at 1024px).
        let radius = scale_to_resolution(smoothing.max(1) as f32, w, h).round().max(1.0) as usize;
        // High detail preservation = small epsilon = the mask tracks edges
        // tightly; low = large epsilon = the mask blurs straight across them.
        let eps = (0.02 + (1.0 - detail) * 0.3).powi(2);

        // Zone coordinate: log2 luminance mapped from [-8 EV .. 0 EV] to [0..1].
        // Below the floor everything collapses to zone 0 (the curve's left edge).
        let luma = luma_values(&data);
        let floor = (2.0f32).powf(-EV_RANGE);
        let zones: Vec<f32> = luma
            .iter()
            .map(|&l| ((l.max(floor).log2() + EV_RANGE) / EV_RANGE).clamp(0.0, 1.0))
            .collect();

        // Edge-preserving smoothing turns the per-pixel zone map into a
        // large-scale exposure mask — this is what makes it dodging/burning
        // rather than a tone curve, and what keeps halos out of the result.
        let mask = guided_filter_plane(&zones, &zones, wu, hu, radius, eps);

        let mut result = (*data).clone();
        for (i, px) in result.pixels_mut().enumerate() {
            // LUT output is the decoded curve value in [0,1]; the flat default
            // decodes to 0.5, i.e. 0 EV.
            let gain_ev = (sample_lut(&lut, mask[i].clamp(0.0, 1.0)) - 0.5) * GAIN_EV_SPAN;
            let gain = (2.0f32).powf(gain_ev);
            for val in px.iter_mut().take(color_ch) {
                *val = (*val * gain).clamp(0.0, 1.0);
            }
            // Alpha (last channel on 2/4-channel images) is left untouched.
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
#[path = "tone_equalizer_tests.rs"]
mod tests;
