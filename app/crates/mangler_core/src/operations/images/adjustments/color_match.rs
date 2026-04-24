//! Color match — per-channel histogram transfer from a reference image.
//!
//! Computes the cumulative distribution function (CDF) of the source and
//! reference images independently per channel, then builds a lookup table
//! mapping each source value to the reference value with the closest CDF.
//! `strength` blends the remapped image with the original.
//!
//! This is per-channel (R, G, B processed independently); the classic
//! "match these two photos' colour" look. Alpha is passed through.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Per-channel histogram matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentColorMatch {}

impl OpImageAdjustmentColorMatch {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "color match".to_string(),
            description: "Remaps the source image so its per-channel histogram matches the reference image.".to_string(),
            help: "Builds a 256-bin cumulative distribution for each colour channel of both source and reference, then constructs a lookup table that maps each source value to the reference value with the closest CDF. Channels are processed independently (R, G, B each get their own LUT), which produces the classic photo colour-match look rather than a true 3D transform.\n\nIf the reference has fewer channels than the source (for example a grayscale reference) its single LUT is fanned out across the source's colour channels. Strength lerps between the original and the fully matched image, letting you dial the effect in softly. Alpha passes through unchanged.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("source".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose per-channel histogram will be remapped."),
            Input::new("reference".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image providing the target histogram the source is matched against."),
            Input::new("strength".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Blend between the original source (0) and the fully matched result (1)."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Source image with its colour distribution shifted toward the reference."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let source_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let reference_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let strength_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: source, change_id: _ } = source_converted.unwrap() else { unreachable!() };
        let Value::Image { data: reference, change_id: _ } = reference_converted.unwrap() else { unreachable!() };
        let Value::Decimal(strength) = strength_converted.unwrap() else { unreachable!() };

        let strength = strength.clamp(0.0, 1.0);
        let src_ch = source.channels() as usize;
        let ref_ch = reference.channels() as usize;
        let has_alpha = src_ch == 2 || src_ch == 4;
        let colour_channels = if has_alpha { src_ch - 1 } else { src_ch };
        let ref_colour_channels = if ref_ch == 2 || ref_ch == 4 { ref_ch - 1 } else { ref_ch };

        // Build one LUT per colour channel. If reference has fewer channels
        // (e.g. grayscale ref, RGB source), we fan out the grayscale LUT
        // across every source channel.
        const BINS: usize = 256;
        let mut luts: Vec<[f32; BINS]> = Vec::with_capacity(colour_channels);
        for c in 0..colour_channels {
            let ref_c = c.min(ref_colour_channels.saturating_sub(1));
            let src_cdf = channel_cdf(&source, c, BINS);
            let ref_cdf = channel_cdf(&reference, ref_c, BINS);
            luts.push(match_cdfs(&src_cdf, &ref_cdf));
        }

        let (width, height) = source.dimensions();
        let mut output = FloatImage::new(width, height, source.channels());
        let mut buf = [0.0f32; 4];
        for y in 0..height {
            for x in 0..width {
                let p = source.get_pixel(x, y);
                for c in 0..src_ch {
                    if has_alpha && c == src_ch - 1 {
                        buf[c] = p[c];
                    } else {
                        let bin = ((p[c].clamp(0.0, 1.0)) * (BINS as f32 - 1.0)).round() as usize;
                        let mapped = luts[c][bin.min(BINS - 1)];
                        buf[c] = p[c] * (1.0 - strength) + mapped * strength;
                    }
                }
                output.put_pixel(x, y, &buf[..src_ch]);
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

/// Compute a normalised cumulative distribution for one channel of an image.
fn channel_cdf(img: &FloatImage, channel: usize, bins: usize) -> Vec<f32> {
    let ch = img.channels() as usize;
    let mut hist = vec![0u32; bins];
    for px in img.pixels() {
        let c = channel.min(ch - 1);
        let v = px[c].clamp(0.0, 1.0);
        let bin = (v * (bins as f32 - 1.0)).round() as usize;
        hist[bin.min(bins - 1)] += 1;
    }
    let total: u32 = hist.iter().sum();
    let total = total.max(1) as f32;
    let mut cdf = vec![0.0f32; bins];
    let mut running = 0u32;
    for (i, &h) in hist.iter().enumerate() {
        running += h;
        cdf[i] = running as f32 / total;
    }
    cdf
}

/// Build a LUT mapping each source bin to the reference value with the
/// closest CDF. Uses a single forward scan over the reference since both
/// CDFs are monotonically increasing.
fn match_cdfs(src: &[f32], reference: &[f32]) -> [f32; 256] {
    let mut out = [0.0f32; 256];
    let mut ref_idx = 0usize;
    let bins = src.len();
    for i in 0..bins {
        let target = src[i];
        while ref_idx + 1 < reference.len() && reference[ref_idx + 1] < target {
            ref_idx += 1;
        }
        out[i] = ref_idx as f32 / (bins as f32 - 1.0);
    }
    out
}

#[cfg(test)]
#[path = "color_match_tests.rs"]
mod tests;
