//! Bloom: luminance-keyed bright-pass glow, blurred and screen-composited
//! back over the source.
//!
//! Unlike inner/outer glow (mask-shape effects driven by dilating/eroding a
//! silhouette), bloom keys on brightness: pixels above `threshold` spill a
//! soft, blurred halo of light back over the image, the way a camera lens or
//! game-engine bloom pass does.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::adjustments::common::smoothstep;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::operations::images::fx::outer_glow::PARALLEL_PIXELS;
use crate::operations::numbers::image::pixel_luma;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Bloom — soft-knee bright-pass, Gaussian spread, screen composite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageFxBloom {}

impl OpImageFxBloom {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bloom".to_string(),
            description: "Soft-knee bright-pass, Gaussian blur, and screen composite — the classic bloom chain.".to_string(),
            help: "Luminance bloom: soft-knee bright-pass, Gaussian spread, screen composite — the classic game-engine bloom chain. Unlike inner/outer glow (mask-shape effects), bloom keys on image brightness.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image; bright regions spill a soft halo back over themselves."),
            Input::new("threshold".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance above which pixels start contributing to the bloom."),
            Input::new("knee".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Softness of the bright-pass cutoff below the threshold; 0 is a hard cutoff, higher values ramp in gradually."),
            Input::new("radius".to_string(), Value::Integer(48), Some(InputSettings::Slider { range: (1.0, 256.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Blur spread in pixels at a 1024px reference (scales with image size); larger values produce a wider, softer halo."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Brightness multiplier applied to the bloom before it's screened over the source; 0 disables the effect entirely."),
            Input::new("tint".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None)
                .with_description("Colour the bloom halo is tinted with."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Source image with the bloom halo screened over it."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let knee_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let tint_converted = convert_input(inputs, 5, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(knee) = knee_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Color(tint) = tint_converted.unwrap() else { unreachable!() };

        // Zero intensity means "no bloom" — hand back the original Arc rather
        // than doing a full blur/composite pass that would end up a no-op.
        if intensity <= 0.0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if ch == 1 || ch == 2 { 1 } else { 3 };

        // knee_width is anchored to threshold.max(0.05) so a near-zero
        // threshold still gets a sane knee width. low_edge <= threshold
        // always holds (knee >= 0), and smoothstep degenerates to a hard
        // step when the two edges coincide (knee == 0), so the ramp
        // direction is never inverted.
        let knee_width = knee.max(0.0) * threshold.max(0.05);
        let low_edge = threshold - knee_width;

        // Bright-pass field: source colour channels weighted by the
        // bright-pass mask. Alpha is data about the source, not bloom
        // contribution, so it's dropped from the field entirely and copied
        // through untouched at composite time.
        let src = data.as_raw();
        let mut field = vec![0.0f32; (width as usize) * (height as usize) * color_ch];

        let build_px = |(dst, px): (&mut [f32], &[f32])| {
            let l = pixel_luma(px);
            let w = smoothstep(low_edge, threshold, l);
            for c in 0..color_ch {
                dst[c] = px[c] * w;
            }
        };
        if src.len() / ch.max(1) >= PARALLEL_PIXELS {
            field.par_chunks_exact_mut(color_ch).zip(src.par_chunks_exact(ch)).for_each(build_px);
        } else {
            field.chunks_exact_mut(color_ch).zip(src.chunks_exact(ch)).for_each(build_px);
        }
        let field_image = FloatImage::from_raw(width, height, color_ch as u32, field).unwrap();

        // radius is authored in reference pixels (at 1024px) and scaled to
        // the actual image so the halo reads the same relative size at any
        // resolution; used directly as the blur sigma.
        let sigma = scale_to_resolution(radius.max(1) as f32, width, height).max(0.0);
        let blurred = gaussian_blur_image(&field_image, sigma);

        let (tr, tg, tb, _ta) = tint.to_srgb_float();
        let tint_channels: [f32; 3] = if color_ch == 1 {
            let l = pixel_luma(&[tr, tg, tb]);
            [l, l, l]
        } else {
            [tr, tg, tb]
        };

        let blurred_raw = blurred.as_raw();
        let mut out = vec![0.0f32; src.len()];

        let composite_px = |((dst, src_px), bloom_px): ((&mut [f32], &[f32]), &[f32])| {
            for c in 0..color_ch {
                let bloom = (bloom_px[c] * tint_channels[c] * intensity).max(0.0);
                dst[c] = (1.0 - (1.0 - src_px[c]) * (1.0 - bloom)).clamp(0.0, 1.0);
            }
            if has_alpha {
                dst[color_ch] = src_px[color_ch];
            }
        };
        if src.len() / ch.max(1) >= PARALLEL_PIXELS {
            out.par_chunks_exact_mut(ch).zip(src.par_chunks_exact(ch)).zip(blurred_raw.par_chunks_exact(color_ch)).for_each(composite_px);
        } else {
            out.chunks_exact_mut(ch).zip(src.chunks_exact(ch)).zip(blurred_raw.chunks_exact(color_ch)).for_each(composite_px);
        }
        let output = FloatImage::from_raw(width, height, ch as u32, out).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "bloom_tests.rs"]
mod tests;
