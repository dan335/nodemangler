//! Vignette operation: darken (or brighten) pixels toward the image edges.
//!
//! Computes a normalized radial distance from the image centre (corner ≈ 1)
//! and multiplies colour channels by a falloff that begins at `radius` and
//! ramps over `softness`. Alpha is preserved.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::tone_curve::{optional_lut, sample_lut, tone_curve_input};
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use super::common::smoothstep;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Radial vignette that scales brightness with distance from centre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentVignette {}

impl OpImageAdjustmentVignette {
    /// Returns the node metadata (name and description) for vignette.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "vignette".to_string(),
            description: "Darkens the image toward its edges with a soft radial falloff.".to_string(),
            help: "For each pixel a normalized distance from the centre is computed in the unit square (0 at the centre, ~1 at the corners). A smoothstep ramp from `radius` to `radius + softness` drives the falloff `t`, and colour channels are multiplied by `1 - amount * t`. Inside `radius` the image is untouched; beyond it brightness rolls off to `1 - amount` at the corners.\n\nAmount controls strength (1 fully darkens the corners to black), radius sets the clean inner region, and softness sets how gradually the darkening ramps in. Alpha is left unchanged. The distance is normalized per axis, so the vignette tracks the image aspect ratio rather than assuming a square.\n\n`falloff` reshapes the smoothstep ramp itself before it drives the darkening (x: 0 = inside radius/untouched, 1 = at the outer edge of the softness band). The default diagonal leaves the plain smoothstep falloff unchanged.".to_string(),
        }
    }

    /// Creates input ports: image, strength, inner radius, and softness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to apply the vignette to."),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Darkening strength; 1 drives the corners fully to black."),
            Input::new("radius".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Normalized inner radius left untouched before falloff starts."),
            Input::new("softness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Width of the falloff ramp from the inner radius outward."),
            tone_curve_input("falloff", "Reshapes the darkening ramp (x: 0 = inside radius, 1 = outer edge of the softness band). Default diagonal leaves the smoothstep falloff unchanged."),
        ]
    }

    /// Creates the output port: the vignetted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Image darkened toward the edges by a radial falloff."),
        ]
    }

    /// Executes the vignette by scaling each colour channel by the radial falloff.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(amount) = 1,
            Decimal(radius) = 2,
            Decimal(softness) = 3,
            Curve(falloff_curve) = 4,
        }
        let lut = optional_lut(&falloff_curve);

        if amount == 0.0 {
            // Zero strength: every pixel is multiplied by 1.0, i.e. untouched.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let (w, h) = data.dimensions();
        let ch = data.channels();
        let color_ch = (if ch == 2 || ch == 4 { ch - 1 } else { ch }) as usize;
        // End of the falloff ramp; nudge above `radius` so a zero-width ramp
        // still produces a hard edge rather than a divide-by-zero.
        let end = (radius + softness).min(1.0).max(radius + 1e-4);

        let mut result = (*data).clone();
        result.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
            // Centre-relative coordinates in [-1, 1] per axis.
            let dx = ((x as f32 + 0.5) / w as f32) * 2.0 - 1.0;
            let dy = ((y as f32 + 0.5) / h as f32) * 2.0 - 1.0;
            // Normalize so the corner sits at ~1.0.
            let dist = (dx * dx + dy * dy).sqrt() / std::f32::consts::SQRT_2;
            let mut t = smoothstep(radius, end, dist);
            if let Some(lut) = &lut {
                t = sample_lut(lut, t);
            }
            let mul = 1.0 - amount * t;
            for val in pixel.iter_mut().take(color_ch) {
                *val *= mul;
            }
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "vignette_tests.rs"]
mod tests;
