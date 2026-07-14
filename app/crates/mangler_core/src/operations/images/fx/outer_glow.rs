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
use crate::operations::images::filter::morphology::erode::separable_morphology;
use crate::operations::images::tone_curve::{optional_lut, sample_lut, tone_curve_input};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Minimum pixel count before the fx passes are parallelized.
pub(crate) const PARALLEL_PIXELS: usize = 1 << 16;

/// Outer glow — bright halo around the outside of a mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageFxOuterGlow {}

impl OpImageFxOuterGlow {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "outer glow".to_string(),
            description: "Glow extending outside a mask — dilate, subtract, blur, tint.".to_string(),
            help: "Collapses the input to a single-channel mask field, dilates it by `radius` pixels using a separable max-morphology pass, and subtracts the original mask to isolate a ring that extends outward from the silhouette. That ring is then Gaussian-blurred with sigma = radius/2 and tinted with the chosen colour.\n\nOutput is an RGBA halo layer whose alpha is glow * intensity * color.a clamped to 0-1, designed to be composited above the source. Intensity can exceed 1 for bloomed looks. Because both the dilation and blur scale with radius, raising the radius expands the halo while keeping its soft-edged character. `radius` is expressed in pixels at a 1024px reference and is scaled to the actual image size, so the glow reads the same at any resolution.\n\n`falloff` remaps the blurred glow strength before it's tinted (0 = beyond the halo, 1 = strongest, right at the mask's outside edge) — a Photoshop-contour-style shaping curve, not a spatial distance field. The default diagonal leaves the glow's natural blur profile unchanged.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Shape whose outside edge the glow radiates from."),
            Input::new("radius".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Dilation distance in pixels at a 1024px reference (scales with image size); larger values extend the glow further outward."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Brightness multiplier applied to the glow's alpha."),
            Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None)
                .with_description("Colour the glow ring is tinted with."),
            tone_curve_input("falloff", "Remaps glow strength before tinting (x: 0 = beyond the halo, 1 = strongest at the outside edge). Default diagonal leaves the glow unchanged."),
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
        let falloff_converted = convert_input(inputs, 4, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = mask_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Curve(falloff_curve) = falloff_converted.unwrap() else { unreachable!() };
        let lut = optional_lut(&falloff_curve);

        let (width, height) = data.dimensions();
        let mask_field = to_mask_field(&data);

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the glow is the same relative size at any resolution.
        let radius = scale_to_resolution(radius.max(1) as f32, width, height).round().max(1.0) as i32;
        let dilated = separable_morphology(&mask_field, radius, |a, b| a.max(b));

        // Ring = dilated - original (clamped to non-negative).
        let ring = subtract_fields(&dilated, &mask_field, width, height);
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

/// Collapse an arbitrary-channel image down to a single-channel mask field.
pub(crate) fn to_mask_field(data: &FloatImage) -> FloatImage {
    let (w, h) = data.dimensions();
    let ch = data.channels() as usize;
    let src = data.as_raw();

    // Extract one scalar per pixel with the channel dispatch hoisted out of
    // the pixel loop.
    fn extract<F: Fn(&[f32]) -> f32 + Sync>(src: &[f32], ch: usize, f: F) -> Vec<f32> {
        if src.len() / ch >= PARALLEL_PIXELS {
            src.par_chunks_exact(ch).map(&f).collect()
        } else {
            src.chunks_exact(ch).map(f).collect()
        }
    }
    let out = match ch {
        1 => src.to_vec(),
        2 => extract(src, ch, |p| p[0] * p[1]),
        3 => extract(src, ch, |p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]),
        _ => extract(src, ch, |p| (0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]) * p[3]),
    };
    FloatImage::from_raw(w, h, 1, out).unwrap()
}

/// Elementwise `(a - b).max(0.0)` over two single-channel fields.
pub(crate) fn subtract_fields(a: &FloatImage, b: &FloatImage, width: u32, height: u32) -> FloatImage {
    let (a_raw, b_raw) = (a.as_raw(), b.as_raw());
    let out: Vec<f32> = if a_raw.len() >= PARALLEL_PIXELS {
        a_raw.par_iter().zip(b_raw.par_iter()).map(|(&av, &bv)| (av - bv).max(0.0)).collect()
    } else {
        a_raw.iter().zip(b_raw.iter()).map(|(&av, &bv)| (av - bv).max(0.0)).collect()
    };
    FloatImage::from_raw(width, height, 1, out).unwrap()
}

/// Paints a single-channel field as an RGBA layer with constant colour `rgb`
/// and alpha `(field * f1 * f2).clamp(0, 1)`.
pub(crate) fn tint_field(field: &FloatImage, rgb: [f32; 3], f1: f32, f2: f32) -> FloatImage {
    let (w, h) = field.dimensions();
    let src = field.as_raw();
    let mut out = vec![0.0f32; src.len() * 4];

    let tint_px = |(dst, &m): (&mut [f32], &f32)| {
        dst[0] = rgb[0];
        dst[1] = rgb[1];
        dst[2] = rgb[2];
        dst[3] = (m * f1 * f2).clamp(0.0, 1.0);
    };
    if src.len() >= PARALLEL_PIXELS {
        out.par_chunks_exact_mut(4).zip(src.par_iter()).for_each(tint_px);
    } else {
        out.chunks_exact_mut(4).zip(src.iter()).for_each(tint_px);
    }
    FloatImage::from_raw(w, h, 4, out).unwrap()
}

#[cfg(test)]
#[path = "outer_glow_tests.rs"]
mod tests;
