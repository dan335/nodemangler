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
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
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
            help: "Collapses the input to a single-channel mask field, erodes it by `radius` pixels using a separable min-morphology pass, and subtracts the eroded result from the original to isolate a ring that hugs the inside edge. That ring is then Gaussian-blurred with sigma = radius/2 and painted with the chosen colour.\n\nOutput is an RGBA layer whose alpha is glow * intensity * color.a clamped to 0-1, ready to composite above the source. Intensity can exceed 1 to saturate the halo. Larger radius values both widen the ring and soften it since the blur scales with radius. `radius` is expressed in pixels at a 1024px reference and is scaled to the actual image size, so the glow reads the same at any resolution.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Shape whose inside edge the glow hugs."),
            Input::new("radius".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Erosion distance in pixels at a 1024px reference (scales with image size); larger values push the glow further inward."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Brightness multiplier applied to the glow's alpha."),
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None)
                .with_description("Colour the glow ring is tinted with."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA layer with a blurred glow ring sitting inside the mask boundary."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let mask_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let color_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = mask_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let mask_field = to_mask_field(&data);

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the glow is the same relative size at any resolution.
        let radius = scale_to_resolution(radius.max(1) as f32, width, height).round().max(1.0) as i32;
        let eroded = separable_morphology(&mask_field, radius, |a, b| a.min(b));

        // Ring = original - eroded (clamped to non-negative).
        let ring = subtract_fields(&mask_field, &eroded, width, height);
        let glow = gaussian_blur_image(&ring, (radius as f32) * 0.5);

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
