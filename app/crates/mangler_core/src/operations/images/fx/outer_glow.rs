//! Outer glow: halo extending outside a mask.
//!
//! Implementation: dilate the mask by `radius`, subtract the original mask,
//! blur the resulting ring, and tint. Output is RGBA with glow colour
//! multiplied by a blurred alpha.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::operations::images::filter::erode::separable_morphology;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Outer glow — bright halo around the outside of a mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageFxOuterGlow {}

impl OpImageFxOuterGlow {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "outer glow".to_string(),
            description: "Glow extending outside a mask — dilate, subtract, blur, tint.".to_string(),
            help: "Collapses the input to a single-channel mask field, dilates it by `radius` pixels using a separable max-morphology pass, and subtracts the original mask to isolate a ring that extends outward from the silhouette. That ring is then Gaussian-blurred with sigma = radius/2 and tinted with the chosen colour.\n\nOutput is an RGBA halo layer whose alpha is glow * intensity * color.a clamped to 0-1, designed to be composited above the source. Intensity can exceed 1 for bloomed looks. Because both the dilation and blur scale with radius, raising the radius expands the halo while keeping its soft-edged character.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Shape whose outside edge the glow radiates from."),
            Input::new("radius".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Dilation distance in pixels; larger values extend the glow further outward."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Brightness multiplier applied to the glow's alpha."),
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None)
                .with_description("Colour the glow ring is tinted with."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA halo layer; composite above the source to place it around."),
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

        let radius = radius.max(1);
        let dilated = separable_morphology(&mask_field, radius, |a, b| a.max(b));

        // Ring = dilated - original (clamped to non-negative).
        let mut ring = FloatImage::new(width, height, 1);
        for y in 0..height {
            for x in 0..width {
                let v = (dilated.get_pixel(x, y)[0] - mask_field.get_pixel(x, y)[0]).max(0.0);
                ring.put_pixel(x, y, &[v]);
            }
        }
        let glow = gaussian_blur_image(&ring, (radius as f32) * 0.5);

        let (cr, cg, cb, ca) = color.to_srgb_float();
        let mut output = FloatImage::new(width, height, 4);
        for y in 0..height {
            for x in 0..width {
                let a = (glow.get_pixel(x, y)[0] * intensity * ca).clamp(0.0, 1.0);
                output.put_pixel(x, y, &[cr, cg, cb, a]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Collapse an arbitrary-channel image down to a single-channel mask field.
pub(crate) fn to_mask_field(data: &FloatImage) -> FloatImage {
    let (w, h) = data.dimensions();
    let ch = data.channels() as usize;
    let mut out = FloatImage::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            let p = data.get_pixel(x, y);
            let v = match ch {
                1 => p[0],
                2 => p[0] * p[1],
                3 => 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2],
                _ => (0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]) * p[3],
            };
            out.put_pixel(x, y, &[v]);
        }
    }
    out
}

#[cfg(test)]
#[path = "outer_glow_tests.rs"]
mod tests;
