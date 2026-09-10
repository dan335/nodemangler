//! Inner glow: halo living inside the edge of a mask.
//!
//! Implementation: erode the mask by `radius`, subtract from the original
//! mask, blur, tint. Output is RGBA with glow colour and a blurred alpha
//! ring that sits inside the mask boundary.

use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::operations::images::filter::morphology::erode::separable_morphology;
use crate::operations::images::fx::outer_glow::{subtract_fields, tint_field, to_mask_field};
use crate::operations::images::tone_curve::{optional_lut, sample_lut, tone_curve_input};
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, scale_to_resolution, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Inner glow — halo along the inside edge of a mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageFxInnerGlow {}

impl OpImageFxInnerGlow {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "inner glow".to_string(),
            description: "Glow along the inside edge of a mask — mask minus erosion, blurred and tinted.".to_string(),
            help: "Collapses the input to a single-channel mask field, erodes it by `radius` pixels using a separable min-morphology pass, and subtracts the eroded result from the original to isolate a ring that hugs the inside edge. That ring is then Gaussian-blurred with sigma = radius/2 and painted with the chosen colour.\n\nOutput is an RGBA layer whose alpha is glow * intensity * color.a clamped to 0-1, ready to composite above the source. Intensity can exceed 1 to saturate the halo. Larger radius values both widen the ring and soften it since the blur scales with radius. `radius` is expressed in pixels at a 1024px reference and is scaled to the actual image size, so the glow reads the same at any resolution.\n\n`falloff` remaps the blurred glow strength before it's tinted (0 = beyond the halo, 1 = strongest, at the mask's inside edge) — a Photoshop-contour-style shaping curve, not a spatial distance field. The default diagonal leaves the glow's natural blur profile unchanged.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("mask")
                .with_description("Shape whose inside edge the glow hugs."),
            Input::new("radius".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Erosion distance in pixels at a 1024px reference (scales with image size); larger values push the glow further inward."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Brightness multiplier applied to the glow's alpha."),
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None)
                .with_description("Colour the glow ring is tinted with."),
            tone_curve_input("falloff", "Remaps glow strength before tinting (x: 0 = beyond the halo, 1 = strongest at the inside edge). Default diagonal leaves the glow unchanged."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("RGBA layer with a blurred glow ring sitting inside the mask boundary."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Integer(radius) = 1,
            Decimal(intensity) = 2,
            Color(color) = 3,
            Curve(falloff_curve) = 4,
        }
        let lut = optional_lut(&falloff_curve);

        let (width, height) = data.dimensions();
        let mask_field = to_mask_field(&data);

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the glow is the same relative size at any resolution.
        let radius = scale_to_resolution(radius.max(1) as f32, width, height).round().max(1.0) as i32;
        let eroded = separable_morphology(&mask_field, radius, |a, b| a.min(b));

        // Ring = original - eroded (clamped to non-negative).
        let ring = subtract_fields(&mask_field, &eroded, width, height);
        let mut glow = gaussian_blur_image(&ring, (radius as f32) * 0.5);
        if let Some(lut) = &lut {
            for v in glow.as_raw_mut() {
                *v = sample_lut(lut, *v);
            }
        }

        let (cr, cg, cb, ca) = color.to_srgb_float();
        let output = tint_field(&glow, [cr, cg, cb], intensity, ca);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "inner_glow_tests.rs"]
mod tests;
