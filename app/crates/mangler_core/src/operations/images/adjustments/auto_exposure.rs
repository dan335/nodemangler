//! Automatic exposure adjustment for images.
//!
//! Measures the scene's log-average Rec.709 luminance, then applies a pure
//! multiplicative gain of `2^stops` so that average maps to a target mid-gray
//! (default 0.18). Result is left unclamped — pair with **tone map** for HDR
//! display compression. Distinct from **auto levels**, which stretches the
//! full histogram into [0, 1].

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Default photographic mid-gray (Reinhard 2002 key / 18% gray).
const DEFAULT_TARGET: f32 = 0.18;


/// Floor under log so pure black does not drive exposure to ±∞.
const LOG_DELTA: f32 = 1e-6;

/// Hard clamp on computed stops so a near-black or clipped frame cannot
/// explode or crush the whole image.
const MAX_STOPS: f32 = 10.0;

/// Automatic exposure: scale the image so log-average luminance hits a target mid-gray.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentAutoExposure {}

impl OpImageAdjustmentAutoExposure {
    /// Returns the node metadata (name, description, and help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "auto exposure".to_string(),
            description: "Automatically sets exposure so average luminance hits a target mid-gray.".to_string(),
            help: "Measures the image's log-average Rec.709 luminance (the same scene key used by \
                   photographic tone mappers), then multiplies every colour channel by \
                   2^exposure so that average maps to the chosen target mid-gray \
                   (default 0.18 ≈ 18% gray).\n\n\
                   Strength blends toward identity: 0 leaves the image unchanged, 1 applies \
                   the full correction. Alpha is never touched. The result is intentionally \
                   left UNCLAMPED — values may exceed 1.0 on HDR data and should be compressed \
                   with a tone map node if you need a display-referred output.\n\n\
                   This is a pure brightness scale, not a histogram stretch (use auto levels \
                   for that). Near-black frames are protected by a stop clamp so the gain \
                   cannot run away. The computed exposure in stops is also emitted so you \
                   can read it, smooth it across frames, or feed it into a manual exposure \
                   node.".to_string(),
        }
    }

    /// Creates the input ports: image, target mid-gray, and blend strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image whose exposure is auto-corrected."),
            Input::new("target".to_string(), Value::Decimal(DEFAULT_TARGET), Some(InputSettings::Slider {
                range: (0.01, 1.0),
                step_by: Some(0.01),
                clamp_to_range: true,
            }), None)
                .with_description("Target mid-gray for the log-average luminance (default 0.18). Higher brightens the result."),
            Input::new("strength".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider {
                range: (0.0, 1.0),
                step_by: Some(0.01),
                clamp_to_range: true,
            }), None)
                .with_description("How fully to apply the correction: 0 = identity, 1 = full auto exposure."),
        ]
    }

    /// Creates the output ports: corrected image and the exposure in stops that was applied.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Image scaled by 2^exposure; alpha preserved, values unclamped."),
            Output::new("exposure".to_string(), Value::Decimal(0.0), None)
                .with_description("Exposure in stops that was applied (after strength). +1 doubles, −1 halves."),
        ]
    }

    /// Executes auto exposure: log-average → stops → multiplicative gain.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(target) = 1,
            Decimal(strength) = 2,
        }

        let target = target.max(1e-6);
        let strength = strength.clamp(0.0, 1.0);

        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        let log_avg = log_average_luminance(data.as_ref(), color_ch);
        // stops such that log_avg * 2^stops = target  ⇒  stops = log2(target / log_avg)
        let raw_stops = (target / log_avg).log2().clamp(-MAX_STOPS, MAX_STOPS);
        let exposure = raw_stops * strength;
        let gain = 2f32.powf(exposure);

        let mut result = (*data).clone();
        if (gain - 1.0).abs() > 1e-7 {
            result.par_pixels_mut().for_each(|pixel| {
                for val in pixel.iter_mut().take(color_ch) {
                    *val *= gain;
                }
            });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {
                    value: Value::Image {
                        data: Arc::new(result),
                        change_id: get_id(),
                    },
                },
                OutputResponse {
                    value: Value::Decimal(exposure),
                },
            ],
        })
    }
}


/// Log-average Rec.709 luminance over the image (geometric mean of luma + delta).
fn log_average_luminance(img: &crate::float_image::FloatImage, color_ch: usize) -> f32 {
    let mut sum_log = 0.0f64;
    let mut count = 0u64;

    for pixel in img.pixels() {
        let lum = if color_ch >= 3 {
            crate::luma::rec709(pixel[0], pixel[1], pixel[2])
        } else {
            pixel[0]
        }
        .max(0.0);
        sum_log += (lum + LOG_DELTA).ln() as f64;
        count += 1;
    }

    if count == 0 {
        return DEFAULT_TARGET;
    }
    (sum_log / count as f64).exp() as f32
}

#[cfg(test)]
#[path = "auto_exposure_tests.rs"]
mod tests;
