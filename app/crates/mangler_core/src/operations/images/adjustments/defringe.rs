//! Defringe: Lightroom-style hue-targeted desaturation of purple/green
//! fringing artefacts at high-contrast edges.
//!
//! Chromatic aberration typically shows up as purple or green fringes right
//! at strong edges. This node finds edges with a Sobel gradient on the
//! image's luminance and desaturates pixels there that fall within the
//! purple or green hue bands, leaving flat regions and other hues alone.
//! A heuristic edge-hue mask, not a physically modelled lens correction.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::operations::numbers::image::luma_values;
use super::common::{hsl_to_rgb, rgb_to_hsl, smoothstep};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Purple fringe hue band, in degrees.
const PURPLE_RANGE: (f32, f32) = (260.0, 330.0);
/// Green fringe hue band, in degrees.
const GREEN_RANGE: (f32, f32) = (90.0, 150.0);
/// Soft margin on each edge of a fringe hue band.
const BAND_MARGIN: f32 = 15.0;

/// Smooth 0..1 plateau weight: ramps up to 1 over `[lo-margin, lo+margin]`,
/// stays at 1 across the band, then ramps back down to 0 over
/// `[hi-margin, hi+margin]`.
#[inline]
fn hue_band_weight(h: f32, (lo, hi): (f32, f32), margin: f32) -> f32 {
    smoothstep(lo - margin, lo + margin, h) * (1.0 - smoothstep(hi - margin, hi + margin, h))
}

/// Sobel gradient magnitude of a planar luma buffer, clamped to [0, 1].
/// Edges are handled by clamping sample coordinates (extend, not wrap).
fn sobel_magnitude(luma: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h];
    if w == 0 || h == 0 {
        return out;
    }
    let sample = |x: i32, y: i32| -> f32 {
        let sx = x.clamp(0, w as i32 - 1) as usize;
        let sy = y.clamp(0, h as i32 - 1) as usize;
        luma[sy * w + sx]
    };
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let xi = x as i32;
            let yi = y as i32;
            let gx = -sample(xi - 1, yi - 1) + sample(xi + 1, yi - 1)
                - 2.0 * sample(xi - 1, yi) + 2.0 * sample(xi + 1, yi)
                - sample(xi - 1, yi + 1) + sample(xi + 1, yi + 1);
            let gy = -sample(xi - 1, yi - 1) - 2.0 * sample(xi, yi - 1) - sample(xi + 1, yi - 1)
                + sample(xi - 1, yi + 1) + 2.0 * sample(xi, yi + 1) + sample(xi + 1, yi + 1);
            // Normalize by the max magnitude a single-axis kernel can produce
            // (4, for a full 0->1 luma step), then clamp to 1.
            let mag = ((gx / 4.0).powi(2) + (gy / 4.0).powi(2)).sqrt();
            row[x] = mag.min(1.0);
        }
    });
    out
}

/// Hue-targeted edge defringe operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDefringe {}

impl OpImageAdjustmentDefringe {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "defringe".to_string(),
            description: "Heuristic hue-targeted edge desaturation (Lightroom-defringe-style) for purple/green fringing.".to_string(),
            help: "Computes a Sobel gradient magnitude on the image's luminance to find edges, then at pixels above `edge threshold` checks whether the pixel's hue falls in the purple (260-330 deg) or green (90-150 deg) band (each toggleable, with a 15 degree soft margin). Matching pixels are desaturated by `s *= 1 - amount * band_weight * edge_weight`, where `edge_weight` ramps in smoothly from `edge threshold` to roughly double it.\n\nThis targets the purple/green fringes chromatic aberration typically produces right at high-contrast edges, without touching flat regions or other hues. `amount` 0, or both `purple`/`green` disabled, leaves the image byte-identical. Grayscale inputs (fewer than 3 channels) have no hue and pass through unchanged; alpha is preserved. Heuristic only — not a lens-calibrated CA correction.".to_string(),
        }
    }

    /// Creates the input ports: image, amount, edge threshold, and the two band toggles.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to defringe."),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Desaturation strength at qualifying edge pixels; 0 leaves the image unchanged."),
            Input::new("edge threshold".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Minimum Sobel edge strength (0-1) for a pixel to be considered for defringing."),
            Input::new("purple".to_string(), Value::Bool(true), None, None)
                .with_description("Target the purple fringe hue band (260-330 degrees)."),
            Input::new("green".to_string(), Value::Bool(true), None, None)
                .with_description("Target the green fringe hue band (90-150 degrees)."),
        ]
    }

    /// Creates the output port: the defringed image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Image with purple/green edge fringing desaturated."),
        ]
    }

    /// Executes the defringe operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(amount) = 1,
            Decimal(threshold) = 2,
            Bool(purple) = 3,
            Bool(green) = 4,
        }

        let amount = amount as f32;
        let threshold = threshold as f32;

        let ch = data.channels() as usize;
        if amount <= 0.0 || (!purple && !green) || ch < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let mut result = (*data).clone();
        let (w, h) = result.dimensions();
        let wu = w as usize;
        let hu = h as usize;

        let luma = luma_values(&result);
        let edges = sobel_magnitude(&luma, wu, hu);

        result
            .par_pixels_mut()
            .zip(edges.par_iter())
            .for_each(|(px, &g)| {
                if g <= threshold {
                    return;
                }
                let edge_w = smoothstep(threshold, threshold * 2.0 + 0.05, g);

                let (hue, s, l) = rgb_to_hsl(px[0], px[1], px[2]);
                let mut band_w = 0.0f32;
                if purple {
                    band_w = band_w.max(hue_band_weight(hue, PURPLE_RANGE, BAND_MARGIN));
                }
                if green {
                    band_w = band_w.max(hue_band_weight(hue, GREEN_RANGE, BAND_MARGIN));
                }
                if band_w <= 0.0 {
                    return;
                }
                let ns = (s * (1.0 - amount * band_w * edge_w)).clamp(0.0, 1.0);
                let (r, g2, b) = hsl_to_rgb(hue, ns, l);
                px[0] = r;
                px[1] = g2;
                px[2] = b;
                // Alpha (channel 3 on 4-channel images) is left untouched.
            });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "defringe_tests.rs"]
mod tests;
