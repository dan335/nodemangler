//! HSL Mixer: independent hue/saturation/lightness adjustment per hue band.
//!
//! Mirrors Lightroom's HSL panel / darktable's color zones module: eight
//! named hue bands (red, orange, yellow, green, aqua, blue, purple, magenta)
//! each get their own hue-shift, saturation, and lightness sliders. A pixel's
//! contribution to a band is weighted by its angular distance from the
//! band's centre hue, smoothly falling to zero at the nearest neighbouring
//! band's centre (bands are not evenly spaced around the wheel).

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use super::common::{hsl_to_rgb, rgb_to_hsl, smoothstep};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Band centre hues in degrees, in the frozen band-major input order.
const BAND_NAMES: [&str; 8] = ["red", "orange", "yellow", "green", "aqua", "blue", "purple", "magenta"];
const BAND_CENTERS: [f32; 8] = [0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 280.0, 320.0];

/// Shortest angular distance between two hues on the 0-360 wheel.
#[inline]
fn hue_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Per-band half-width: the angular distance to the nearest neighbouring
/// band centre (bands are non-uniformly spaced, so this is precomputed per
/// band rather than assumed constant).
fn band_half_widths() -> [f32; 8] {
    let n = BAND_CENTERS.len();
    let mut half_widths = [0.0f32; 8];
    for i in 0..n {
        let prev = BAND_CENTERS[(i + n - 1) % n];
        let next = BAND_CENTERS[(i + 1) % n];
        let c = BAND_CENTERS[i];
        half_widths[i] = hue_distance(c, prev).min(hue_distance(c, next));
    }
    half_widths
}

/// Independent per-hue-band HSL mixer (Lightroom HSL / darktable color zones).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHslMixer {}

impl OpImageAdjustmentHslMixer {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hsl mixer".to_string(),
            description: "Independent hue/saturation/lightness sliders per hue band, Lightroom HSL / darktable color zones style.".to_string(),
            help: "Eight hue bands (red, orange, yellow, green, aqua, blue, purple, magenta) each expose a hue-shift, saturation, and lightness slider. Every pixel is converted to HSL; each band contributes a weight based on the pixel's angular distance from that band's centre hue, smoothly fading to zero by the nearest neighbouring band's centre (the bands are not evenly spaced, so each band's falloff width is derived from its actual neighbours). The weighted deltas from every band are summed: hue shifts add directly (in degrees), saturation deltas apply as a multiplier `(1 + weight * band_saturation)`, and lightness deltas add `weight * band_lightness * 0.5`.\n\nAchromatic pixels (saturation ~0) have no hue to target and are left untouched. All 24 band sliders default to 0, in which case the image is byte-identical to the input. Grayscale inputs (fewer than 3 channels) have no hue and pass through unchanged; alpha is preserved.\n\nThe 24 band inputs are hidden from the graph canvas (config-only, edited in the node settings panel) to keep the node itself compact.".to_string(),
        }
    }

    /// Creates the 25 inputs: image, then 8 bands x (hue, saturation,
    /// lightness) in band-major order. Indices 1..=24 are a frozen
    /// positional contract; new bands (if ever added) must append, never
    /// insert.
    pub fn create_inputs() -> Vec<Input> {
        let mut inputs = vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to adjust."),
        ];
        for name in BAND_NAMES {
            inputs.push(
                Input::new(
                    format!("{name} hue"),
                    Value::Decimal(0.0),
                    Some(InputSettings::Slider { range: (-60.0, 60.0), step_by: Some(1.0), clamp_to_range: true }),
                    None,
                )
                .with_description(format!("Hue shift in degrees applied to the {name} band."))
                .hidden_in_graph(),
            );
            inputs.push(
                Input::new(
                    format!("{name} saturation"),
                    Value::Decimal(0.0),
                    Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                    None,
                )
                .with_description(format!("Saturation multiplier delta applied to the {name} band."))
                .hidden_in_graph(),
            );
            inputs.push(
                Input::new(
                    format!("{name} lightness"),
                    Value::Decimal(0.0),
                    Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                    None,
                )
                .with_description(format!("Lightness delta applied to the {name} band."))
                .hidden_in_graph(),
            );
        }
        inputs
    }

    /// Creates the output port: the mixed image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with per-hue-band HSL adjustments applied."),
        ]
    }

    /// Executes the HSL mixer over all eight bands.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        // hue[i], sat[i], light[i] for band i, read from the frozen band-major layout.
        let mut hue = [0.0f32; 8];
        let mut sat = [0.0f32; 8];
        let mut light = [0.0f32; 8];
        for i in 0..8 {
            let base = 1 + i * 3;
            if let Some(Value::Decimal(v)) = convert_input(inputs, base, ValueType::Decimal, &mut input_errors) {
                hue[i] = v as f32;
            }
            if let Some(Value::Decimal(v)) = convert_input(inputs, base + 1, ValueType::Decimal, &mut input_errors) {
                sat[i] = v as f32;
            }
            if let Some(Value::Decimal(v)) = convert_input(inputs, base + 2, ValueType::Decimal, &mut input_errors) {
                light[i] = v as f32;
            }
        }

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let ch = data.channels() as usize;
        let all_zero = hue.iter().all(|v| *v == 0.0) && sat.iter().all(|v| *v == 0.0) && light.iter().all(|v| *v == 0.0);
        if ch < 3 || all_zero {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let half_widths = band_half_widths();

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let (h, s, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
            if s <= 1e-4 {
                // Achromatic: no hue to target, leave untouched.
                continue;
            }

            let mut hue_delta = 0.0f32;
            let mut sat_mult = 1.0f32;
            let mut light_delta = 0.0f32;
            for i in 0..8 {
                let d = hue_distance(h, BAND_CENTERS[i]);
                let weight = 1.0 - smoothstep(0.0, half_widths[i], d);
                if weight <= 0.0 {
                    continue;
                }
                hue_delta += weight * hue[i];
                sat_mult *= 1.0 + weight * sat[i];
                light_delta += weight * light[i] * 0.5;
            }

            let nh = (h + hue_delta).rem_euclid(360.0);
            let ns = (s * sat_mult).clamp(0.0, 1.0);
            let nl = (l + light_delta).clamp(0.0, 1.0);
            let (r, g, b) = hsl_to_rgb(nh, ns, nl);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            // Alpha (channel 3 on 4-channel images) is left untouched.
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "hsl_mixer_tests.rs"]
mod tests;
